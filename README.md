# assinafy — SDK Rust

*Português · [Read in English](README.en.md)*

[![Crate](https://img.shields.io/crates/v/assinafy.svg)](https://crates.io/crates/assinafy)
[![Docs](https://docs.rs/assinafy/badge.svg)](https://docs.rs/assinafy)

Cliente Rust assíncrono para todas as operações da [API Assinafy](https://api.assinafy.com.br/v1/docs), além das rotas de compatibilidade de templates. Cada método público documenta rota, requisição, resposta, autenticação e modelos tipados no [docs.rs](https://docs.rs/assinafy).

## Instalação

Suporta Rust 1.86+ e edition 2024. Rust não possui canal LTS; o CI testa o MSRV e o stable atual. HTTPS usa TLS 1.2 ou superior.

```toml
[dependencies]
assinafy = "3.10"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
```

A feature padrão é `rustls-tls`; use `default-features = false, features = ["native-tls"]` para a pilha TLS do sistema operacional.

## Início rápido

```rust,no_run
use assinafy::Client;

#[tokio::main]
async fn main() -> assinafy::Result<()> {
    let client = Client::builder()
        .api_key(std::env::var("ASSINAFY_API_KEY").expect("ASSINAFY_API_KEY"))
        .build()?;
    let signers = client
        .signers("acc_1234567890abcdef12345678")
        .list()
        .per_page(50)
        .send()
        .await?;
    for signer in signers.data {
        println!("{}", signer.full_name);
    }
    Ok(())
}
```

Crie um único `Client` clonável e use seus recursos: `accounts_api`, `account`, `api_keys`, `users`, `signers`, `signer_self`, `documents`, `envelopes`, `embedded_signing`, `assignments`, `contracts`, `tags`, `fields`, `templates`, `webhooks`, `activities`, `public`, `auth_api` e `oauth`. As chamadas retornam `assinafy::Result<T>` e extraem o `data` do envelope `{status,message,data}`; metadados e tokens OAuth são planos.

## Autenticação e ambientes

- `Auth::ApiKey` / `ClientBuilder::api_key`: `X-Api-Key`, para integrações servidor-servidor.
- `Auth::Bearer`: token de usuário ou OAuth em `Authorization`.
- `Auth::AccessToken`: formato legado `?access-token=`.
- `Auth::AccessCode`: somente rotas do signatário, como `?signer-access-code=`. Não pode ser obtido programaticamente.

Produção é o padrão. Para sandbox, use `BaseUrl::custom("https://sandbox.assinafy.com.br/v1")`. URLs personalizadas fora de loopback devem ser HTTPS e não podem conter credenciais, query ou fragmento. Segredos e URLs privadas de download/captura são ocultados em `Debug`.

## OAuth 2.1 com PKCE

Cadastre um callback HTTPS exato em Configurações → Aplicações OAuth. Clientes Public não têm segredo; servidores protegidos usam Confidential e adicionam `.client_secret(...)`. Peça somente os escopos necessários; inclua `account:read` para identificar a conta, `openid` para userinfo e `offline_access` apenas quando precisar de refresh.

1. Leia `oauth().protected_resource_metadata()` e depois `authorization_server_metadata(...)`.
2. Gere novos `PkceChallenge`, state e nonce OIDC opcional; guarde-os na sessão inicial para uma tentativa.
3. Monte a URL com `AuthorizationRequest`; valide origem/caminho do callback, parâmetros duplicados, `state` e `iss` antes de tratar código ou erro.
4. Troque o código de uso único e validade de 60 segundos uma vez com `TokenRequest::authorization_code`. Clientes Confidential adicionam o segredo.
5. Valide qualquer ID token com uma biblioteca OIDC: assinatura/JWKS, emissor, audiência, expiração e nonce. O SDK não faz essa validação.
6. Use `Auth::Bearer`, chame `accounts_api().list()` e persista o único ID de conta, tokens criptografados, expiração e escopos retornados.
7. Serialize refreshes por conexão. Cada refresh rotaciona o token por mais 30 dias; salve atomicamente o novo refresh token antes de usar o access token. Nunca repita um refresh incerto nem reutilize token aposentado.
8. Revogue o refresh token atual uma vez com `RevokeRequest` antes de excluir a conexão.

Token e revogação usam form encoding, não seguem redirects e são enviados uma vez. `offline_access` não volta no `scope` concedido. OAuth funciona somente em produção.

## Fluxo completo de documento e assinatura

1. Crie ou reutilize signatários com `signers(account).create(...)`; informe o contato do canal escolhido e `government_id` quando necessário.
2. Envie PDF (≤25 MB, ≤2000 páginas) com `documents().upload(...)` ou instancie template com IDs existentes ou contatos inline por email/nome.
3. Consulte `documents().get(...)` com prazo máximo até `metadata_ready`; persista IDs para retomar recursos existentes.
4. Adicione tags por **nome**, posicione campos e chame `assignments().estimate_cost(...)`.
5. Crie assignment `Virtual` ou `Collect`. Não repita criação de documento ou assignment após resposta incerta.
6. Entregue cada URL privada. O signatário aceita termos, confirma dados, conclui a verificação, preenche seus itens e assina.
7. Aguarde webhook verificado ou polling limitado; baixe `ArtifactName::Certificated`, `Pades`, `CertificatePage` ou `Bundle`.

Pares permitidos de verificação/notificação:

| Verificação | Notificação | Observação |
| --- | --- | --- |
| `Email` | `Email` | OTP gratuito; padrão. |
| `Whatsapp` | `Whatsapp` | OTP; notificação depende do plano e tem custo. |
| `DigitalCertificate` | `Email` ou `Whatsapp` | Certificado ICP-Brasil A1/A3 do signatário; somente virtual; produz PAdES. |
| `VerificationMethod::id_document_face_match_basic()` | `Email` ou `Whatsapp` | Captura hospedada de documento/face com consentimento explícito. |

Use exatamente um canal de notificação. Verificação/notificação ausente usa padrão ou é inferida pela API. `DigitalCertificate` é rejeitado localmente em `Collect`; papéis de certificado em templates não podem ter campos posicionados do signatário. Estime antes da criação porque planos e preços variam.

### Conclusão biométrica e por certificado

O cliente do signatário deve carregar somente `Auth::AccessCode`. Para biometria, leia `signer_self().identity_verification()`, chame `start_identity_verification(true)` apenas após consentimento explícito e quando `can_start`, e abra a URL bearer retornada sem alterá-la—incluindo o fragmento. Nunca registre essa URL nem a persista no browser. Consulte com prazo: `Expired` pode reiniciar; `Declined` ou `Abandoned` exige novo documento e assignment.

`InReview` permite assinar, mas segura a certificação. A conta lê `assignments().identity_verification(...)`, baixa somente as evidências nomeadas e envia uma `IdentityReviewDecision` explícita. Aprovação libera a certificação após todas as assinaturas; rejeição cancela o documento. Imagens desaparecem após a revisão.

A conclusão A1/A3 exige o certificado do próprio signatário e a extensão Web PKI. O handshake start/complete, disponível somente em produção, fica fora do contrato OpenAPI publicado e deste SDK de servidor; o SDK configura, estima, cria e acompanha assignments por certificado e baixa seu artefato PAdES.

## Templates, contratos e webhooks

A criação por template aceita IDs existentes ou contatos inline com `TemplateDocumentSigner::from_email(role, email, name)`. `estimate_cost` projeta apenas campos de preço. CRUD/download de página de templates permanece como compatibilidade porque funciona ao vivo apesar de ausente no OpenAPI.

`contracts(account)` cobre listar, ler/atualizar, família relacionada, renovar e encerrar. Campos anuláveis diferenciam omitido/valor/JSON `null`; envie o último `updated_at` para concorrência otimista. Nunca repita writes automaticamente. Writes podem depender do plano mesmo quando reads funcionam.

Para webhooks assinados, crie endpoint com `signing_enabled(true)`, criptografe o `secret` retornado uma única vez e verifique o corpo **bruto** antes de parsear com `verify_webhook_signature`. Rejeite IDs de webhook repetidos no armazenamento da aplicação. Apps OAuth não podem ler um segredo existente, mas podem rotacioná-lo com `webhooks:write`. `rotate_endpoint_secret_with_overlap(id, true)` mantém a chave anterior por 24 horas; armazene a nova chave antes de trocar a verificação. Nunca altere endpoint existente para testar.

## Envelopes, assinatura incorporada e notificações

Use `envelopes(account)` para estimar e enviar até 20 documentos preparados em um convite por signatário, depois ler, renomear, cancelar, adicionar/remover documentos, reenviar lembretes, consultar atividades ou baixar o ZIP. Cada documento indica seus signatários e campos virtual/collect. Certificado e biometria continuam disponíveis. Persista IDs e use `Idempotency-Key` da criação ao retomar a mesma requisição.

Opções de criação de assignment/template e corpos de envelope aceitam `max_total_credits`; a recusa `CreditCeilingExceeded` não cria nem cobra nada. Criação por template e de envelope aceita chave de idempotência.

Para assinatura incorporada, configure origens exatas com `embedded_signing()`, marque o signatário de assignment/template como embedded, crie uma sessão curta e carregue sua URL no iframe. A troca é de uso único, inclusive em falha: nunca a repita. Mantenha URL e código de acesso privados; signatários embedded continuam recebendo o convite normal. Iframes aceitam verificação Email/Whatsapp; biometria e certificado usam o fluxo de assinatura hospedado.

Proprietários configuram `account(id).update_notification_policy(...)`; usuários definem preferências de email e push separadamente. Clientes do signatário podem carregar `theme()`/`download_logo()` e solicitar cópia certificada; links expirados devem ser renovados pela operação de reenvio.

## Queries, erros e compatibilidade

Builders de lista expõem `search`, `sort`, filtros, `page` e `per_page` suportados. A lista de assignments e a busca leve de documentos aceitam `sort`. Resultados usam `Page<T>` com metadados de paginação e rate limit.

`Error::Api` preserva status HTTP, mensagem, `code()` de recusa e erros de campo, dados estruturados, `Retry-After`, headers de limite e códigos OAuth. Erros de decode informam categoria/local/tamanho, nunca o corpo. IDs de caminho não podem redirecionar a rota; redirects binários autenticados mantêm credenciais somente em HTTPS seguro e mesma origem.

Valores desconhecidos são preservados onde o modelo público oferece `Other`/`Unknown`. O SDK mantém comportamentos compatíveis ao vivo, como `accountId` de assignments, extras de update de campos, rotas antigas de webhook e CRUD de templates; operações funcionais não são removidas porque um deployment ou plano as recusa.

## Validação

Todos os testes commitados são offline; o teste de conexão OAuth é ignorado e interativo.

```bash
cargo fmt --all --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo test --doc --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --locked
cargo build --all-targets --no-default-features --features native-tls --locked
```

Execute também os testes all-target/all-feature no Rust 1.86. OTP, captura biométrica e conclusão por certificado exigem uma pessoa e não podem ser comprovados pela suíte offline.

## Licença

[MIT](LICENSE)
