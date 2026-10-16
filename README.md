# assinafy — SDK Rust

*Português · [Read in English](README.en.md)*

[![Crate](https://img.shields.io/crates/v/assinafy.svg)](https://crates.io/crates/assinafy)
[![Docs](https://docs.rs/assinafy/badge.svg)](https://docs.rs/assinafy)

Cliente Rust assíncrono e idiomático para a API de assinatura eletrônica da
[Assinafy](https://assinafy.com.br) — plataforma brasileira de assinatura de documentos.

Cobre toda a superfície REST pública documentada em <https://api.assinafy.com.br/v1/docs>:
contas, signatários, documentos, assignments, templates, tags, campos, webhooks, atividades,
chaves de API, OAuth 2.1 e os endpoints voltados ao signatário.

## Sumário

- [Instalação](#instalação)
- [Início rápido](#início-rápido)
- [Como o SDK é organizado](#como-o-sdk-é-organizado)
- [Autenticação](#autenticação)
  - [Chave de API](#chave-de-api)
  - [Token de usuário](#token-de-usuário)
  - [OAuth 2.1 com PKCE](#oauth-21-com-pkce)
  - [Código de acesso do signatário](#código-de-acesso-do-signatário)
- [Ambientes](#ambientes)
- [Fluxo completo do documento](#fluxo-completo-do-documento)
- [Métodos de verificação do signatário](#métodos-de-verificação-do-signatário)
- [Verificação biométrica de identidade](#verificação-biométrica-de-identidade)
- [Certificado digital ICP-Brasil (A1/A3)](#certificado-digital-icp-brasil-a1a3)
- [Fluxo do signatário](#fluxo-do-signatário)
- [Templates](#templates)
- [Tags](#tags)
- [Campos personalizados](#campos-personalizados)
- [Webhooks](#webhooks)
- [Atividades e artefatos](#atividades-e-artefatos)
- [Paginação](#paginação)
- [Erros e limites de taxa](#erros-e-limites-de-taxa)
- [Índice de operações](#índice-de-operações)
- [Features do Cargo](#features-do-cargo)
- [Testes](#testes)
- [Licença](#licença)

## Instalação

Requer Rust 1.86 ou superior e usa a edição Rust 2024. A CI testa tanto o mínimo declarado
(1.86) quanto o stable atual — Rust não tem canal LTS. O cliente HTTP padrão exige TLS 1.2 ou superior.

```toml
[dependencies]
assinafy = "3"
tokio    = { version = "1", features = ["macros", "rt-multi-thread", "time"] }
```

## Início rápido

```rust,no_run
use assinafy::Client;

#[tokio::main]
async fn main() -> assinafy::Result<()> {
    let client = Client::builder()
        .api_key(std::env::var("ASSINAFY_API_KEY").unwrap())
        .build()?;

    let signatarios = client
        .signers("acc_1234567890abcdef12345678")
        .list()
        .per_page(50)
        .send()
        .await?;

    for s in &signatarios.data {
        println!("{} <{:?}>", s.full_name, s.email);
    }
    Ok(())
}
```

## Como o SDK é organizado

Um único `Client` expõe um módulo por área da API. O handle é barato de criar a cada chamada e
empresta do cliente; o próprio `Client` é `Clone` (estado interno com contagem de referências),
então construa um na inicialização e reutilize-o.

| Área | Acessor |
| ---- | ------- |
| Autenticação (login, senha, social) | `Client::auth_api` |
| OAuth 2.1 / OpenID Connect | `Client::oauth` |
| Contas | `Client::accounts_api` / `Client::account` |
| Chaves de API | `Client::api_keys` |
| Usuário autenticado | `Client::users` |
| Signatários | `Client::signers` |
| Signatário (self) | `Client::signer_self` |
| Documentos | `Client::documents` |
| Assignments | `Client::assignments` |
| Tags | `Client::tags` |
| Campos | `Client::fields` |
| Templates | `Client::templates` |
| Webhooks | `Client::webhooks` |
| Atividades | `Client::activities` |
| Endpoints públicos | `Client::public` |

Toda chamada é `async` e devolve `Result<T, Error>`. As respostas usam o mesmo envelope da
API (`{ status, message, data }`), do qual o SDK entrega apenas o `data` já tipado — exceto os
endpoints OAuth, que por especificação respondem com um objeto plano.

## Autenticação

O SDK aceita os quatro esquemas de credencial da API. Troque a credencial em tempo de execução
com `Client::with_auth`.

### Chave de API

Enviada no cabeçalho `X-Api-Key`. É o padrão para integrações servidor-a-servidor.

```rust
use assinafy::Client;

fn criar_cliente() -> assinafy::Result<Client> {
    Client::builder().api_key("chave-de-api").build()
}
```

### Token de usuário

Obtido por `login` e enviado como `Authorization: Bearer <token>`.

```rust,no_run
use assinafy::{Auth, Client};
use assinafy::resources::LoginBody;

async fn entrar() -> assinafy::Result<Client> {
    let anonimo = Client::builder().build()?;
    let sessao = anonimo
        .auth_api()
        .login(&LoginBody::new("usuario@example.invalid", "senha"))
        .await?;

    // `LoginResult` também traz o usuário e as contas que ele enxerga.
    Ok(anonimo.with_auth(Auth::Bearer(sessao.access_token)))
}
```

Há ainda a forma legada por query string (`?access-token=...`), disponível via
`ClientBuilder::access_token`, para integrações que a exijam.

### OAuth 2.1 com PKCE

Use OAuth quando a aplicação age **no workspace de um usuário, com a permissão dele** — ao
contrário da chave de API e do token de usuário, que autenticam o workspace ou o usuário
diretamente. O token resultante carrega apenas os escopos aprovados, vale para um workspace e
nunca alcança faturamento, ciclo de vida da conta, gestão de credenciais ou superfícies
administrativas, quaisquer que sejam seus escopos.

Cadastre o aplicativo no [Assinafy](https://app.assinafy.com.br), em **Configurações → Aplicações
OAuth → Nova aplicação**. O workspace proprietário precisa de um plano elegível e seu proprietário
deve fazer o cadastro. Selecione somente as permissões necessárias. Cadastre um callback HTTPS por
ambiente; a correspondência é exata, inclusive a barra final. O desenvolvimento local exige um túnel
HTTPS. Escolha **Confidencial** para um servidor capaz de proteger um segredo, ou **Público** para
um cliente que não pode fazê-lo. Clientes confidenciais adicionam `.client_secret(segredo)` a cada
pedido de token e revogação.

Guarde cada conexão por usuário da sua aplicação e workspace Assinafy, junto com os escopos
concedidos, a validade e os tokens criptografados. Mantenha PKCE e estado aleatório na sessão do
navegador que iniciou o fluxo, expire-os ao terminar a tentativa e rejeite parâmetros duplicados no
callback. Códigos expiram em 60 segundos e só podem ser trocados uma vez. O SDK devolve o `id_token`
sem verificá-lo: uma biblioteca OpenID Connect deve validar sua assinatura com o `jwks_uri` da
descoberta, emissor, audiência, validade e eventual nonce enviado por `.nonce(valor)`.

O fluxo é o *authorization code* com PKCE **obrigatório** (S256). O SDK cobre todas as etapas,
exceto o redirecionamento do navegador:

```text
1. descoberta      GET /.well-known/oauth-protected-resource
                   GET {issuer}/.well-known/oauth-authorization-server
2. autorização     navegador → {authorization_endpoint}?code_challenge=...
3. consentimento   usuário aprova → redirect_uri?code=...&state=...&iss=...
4. troca           POST /v1/oauth/token   (code + code_verifier)
5. uso             Authorization: Bearer <access_token>
                   POST /v1/oauth/token   (refresh_token)
                   POST /v1/oauth/revoke
```

Gere um par PKCE e um `state` novos a cada tentativa. No retorno ao `redirect_uri`, antes de
qualquer outra coisa — inclusive quando vier `error=...` —, confira se `state` é o valor guardado
e se `iss` é `https://auth.assinafy.com.br`; se algum diferir, pare: a resposta não é sua.

```rust,no_run
use assinafy::resources::{AuthorizationRequest, PkceChallenge, TokenRequest, scope};
use assinafy::{Auth, Client};

async fn autorizar(client_id: &str, redirect_uri: &str) -> assinafy::Result<()> {
    let client = Client::builder().build()?;

    // 1. Descoberta: a API nomeia seu servidor de autorização, que por sua vez
    //    publica o endpoint de autorização usado pelo navegador.
    let recurso = client.oauth().protected_resource_metadata().await?;
    let servidor = client
        .oauth()
        .authorization_server_metadata(&recurso.authorization_servers[0])
        .await?;

    // 2. Par PKCE e `state` novos a cada tentativa; guarde os dois até o callback.
    let pkce = PkceChallenge::generate()?;
    let state = PkceChallenge::generate()?.verifier().to_owned();
    let url = AuthorizationRequest::new(client_id, redirect_uri, &pkce)
        .scopes([scope::DOCUMENTS_READ, scope::ACCOUNT_READ, scope::OPENID, scope::OFFLINE_ACCESS])
        .state(&state)
        .resource(&recurso.resource)
        .url(&servidor.authorization_endpoint)?;
    println!("abra {url}");

    // 4. Conferidos `state` e `iss`, troque o `code` que voltou no redirecionamento,
    //    repetindo o `resource` enviado no passo 2.
    let token = client
        .oauth()
        .token(
            &TokenRequest::authorization_code(
                client_id,
                "code-do-redirecionamento",
                redirect_uri,
                &pkce,
            )
            .resource(&recurso.resource),
        )
        .await?;

    // 5. Passe a agir pelo usuário. O token alcança um único workspace, o único que
    //    `GET /v1/accounts` devolve: guarde o id dele junto dos tokens.
    let como_usuario = client.with_auth(Auth::Bearer(token.access_token.clone()));
    let workspace = &como_usuario.accounts_api().list().await?[0].id;
    let quem = como_usuario.oauth().userinfo().await?;
    println!("agindo por {} no workspace {workspace}", quem.sub);
    Ok(())
}
```

Para serviços internos Assinafy, `TokenRequest::token_exchange` aceita o grant RFC 8693,
com `.requested_token_type("urn:ietf:params:oauth:token-type:access_token")` opcional.
A resposta contém `issued_token_type` e não contém refresh token. Aplicações comuns
recebem `invalid_client` para esse grant e usam o fluxo de código de autorização acima.

Escopos disponíveis (constantes em `assinafy::resources::scope`):

| Escopo | Concede |
| --- | --- |
| `documents:read` | Ler documentos, páginas, tags, signatários, assignments e atividades |
| `documents:write` | Criar, atualizar e excluir documentos e gerir seus signatários e assignments |
| `templates:read` | Ler templates, páginas, papéis, campos e tags |
| `templates:write` | Criar, atualizar e excluir templates e seus componentes |
| `account:read` | Ler perfil, tema e logo do workspace |
| `webhooks:write` | Criar, alterar e excluir os endpoints de webhook do workspace |
| `openid` | Identificar o usuário autenticado e habilitar `/oauth/userinfo` |
| `profile` | Incluir o nome do usuário no `id_token`/userinfo |
| `email` | Incluir o e-mail e sua verificação no `id_token`/userinfo |
| `offline_access` | Emitir refresh token — só para clientes que pedem explicitamente |

Peça sempre o conjunto mínimo. Um escopo ausente responde `403` com
`WWW-Authenticate: Bearer error="insufficient_scope"` nomeando o que falta; o SDK entrega esses
escopos em `Error::insufficient_scope`. Reconecte pedindo-os — repetir a chamada não resolve. Um
`403` sem esse desafio indica outro workspace, o papel do usuário ou uma área que tokens OAuth
nunca alcançam.

Renovação: o access token dura 1 hora e, com `offline_access`, é renovado sem o usuário. Cada
renovação devolve um refresh token **novo** e aposenta o anterior, e reutilizar um refresh token
aposentado encerra a conexão inteira. Por isso, salve o novo refresh token antes de qualquer outra
coisa e faça uma renovação por vez em cada conexão. O SDK envia cada pedido de token uma única
vez — nunca o repete nem segue redirecionamentos — e rejeita uma renovação bem-sucedida que não
traga um refresh token novo.

Quando uma renovação falha sem resposta definitiva — timeout, conexão interrompida, `5xx`, sucesso
sem refresh token novo —, suponha que o token enviado foi aposentado. Releia o token guardado: se
outro worker salvou um diferente, siga com ele; se continua o mesmo, **nunca o envie de novo** —
marque a conexão como desconectada e peça ao usuário para conectar de novo. Só as falhas que
comprovadamente ocorreram antes do envio — resolução de DNS, conexão recusada, handshake TLS: um
`Error::Http` cujo `is_connect()` é verdadeiro — deixam o token intacto e podem ser repetidas.

Um refresh token vale 30 dias, e cada renovação devolve um novo com mais 30 dias: a conexão só
expira se passar 30 dias sem renovação.

As falhas do endpoint de token seguem o objeto plano `{ error, error_description }` da RFC 6749
§5.2. O SDK preserva ambos: a descrição vira a mensagem do erro e o código fica acessível em
`ApiError::oauth_error`.

```rust,no_run
use assinafy::Client;
use assinafy::models::TokenResponse;
use assinafy::resources::TokenRequest;

/// Renova o access token. `salvar` grava de forma durável o refresh token novo da
/// resposta; se falhar, o erro é devolvido e o access token não é usado.
async fn renovar(
    client: &Client,
    client_id: &str,
    refresh_token: &str,
    salvar: impl FnOnce(&TokenResponse) -> std::io::Result<()>,
) -> assinafy::Result<Option<String>> {
    match client
        .oauth()
        .token(&TokenRequest::refresh_token(client_id, refresh_token))
        .await
    {
        Ok(novo) => {
            // O refresh token enviado acaba de ser aposentado, e uma renovação bem-sucedida
            // sempre traz um novo: salve-o antes de usar o access token.
            salvar(&novo)?;
            Ok(Some(novo.access_token))
        }
        // A conexão acabou (refresh token já usado, 30 dias sem renovação, revogado ou
        // nova aprovação com outras permissões): marque-a como desconectada e peça ao
        // usuário para conectar de novo, sem repetir a renovação.
        Err(e) if e.oauth_error() == Some("invalid_grant") => Ok(None),
        // Qualquer outra falha pode ter aposentado o token enviado: a menos que
        // `is_connect()` mostre que o pedido nem saiu, releia o token guardado e nunca o
        // reenvie inalterado.
        Err(outro) => Err(outro),
    }
}
```

Em um `401` da API, renove uma vez; se a renovação falhar, peça ao usuário para conectar de novo.

Para desconectar, revogue o refresh token atual e só então apague os tokens guardados:

```rust,no_run
use assinafy::Client;
use assinafy::resources::RevokeRequest;

async fn desconectar(
    client: &Client,
    client_id: &str,
    refresh_token: &str,
) -> assinafy::Result<()> {
    // A revogação responde 200 para qualquer resultado — inclusive token
    // inexistente —, então nunca serve para sondar se um token existe.
    client
        .oauth()
        .revoke(&RevokeRequest::refresh_token(client_id, refresh_token))
        .await
}
```

> Os endpoints OAuth existem **somente em produção**. Outras implantações respondem
> `404 Página não encontrada.` e não publicam `/.well-known/oauth-protected-resource`.

### Código de acesso do signatário

Os endpoints voltados ao signatário usam o código de acesso entregue fora de banda (e-mail ou
WhatsApp), enviado como `?signer-access-code=...`.

```rust
use assinafy::{Auth, Client};

fn como_signatario(client: &Client) -> Client {
    client.with_auth(Auth::AccessCode("codigo-do-signatario".into()))
}
```

## Ambientes

| Ambiente | Base URL | Builder |
| --- | --- | --- |
| Produção | `https://api.assinafy.com.br/v1` | padrão, ou `.production()` |
| Outra implantação | qualquer URL HTTPS | `.base_url(BaseUrl::custom(...)?)` |

`BaseUrl::custom` aponta para qualquer outro deployment. URLs personalizadas exigem HTTPS
(HTTP só para loopback), sem credenciais embutidas, query string ou fragmento.

```rust
use assinafy::{BaseUrl, Client};

fn cliente_local() -> assinafy::Result<Client> {
    Client::builder()
        .base_url(BaseUrl::custom("http://127.0.0.1:8080/v1")?)
        .api_key("chave-de-teste")
        .build()
}
```

## Fluxo completo do documento

O caminho ponta a ponta de uma assinatura, do upload ao PDF certificado:

```text
 1. criar signatário      POST /accounts/{conta}/signers
 2. enviar o PDF          POST /accounts/{conta}/documents        → status: uploaded
 3. aguardar metadados    GET  /documents/{id}                    → status: metadata_ready
 4. estimar o custo       POST /documents/{id}/assignments/estimate-cost
 5. pedir assinaturas     POST /documents/{id}/assignments        → status: pending_signature
                          (a API notifica cada signatário e devolve signing_urls)
 6. o signatário assina   aceita termos → confirma dados → OTP, biometria ou A1/A3
                          (biometria InReview: a conta decide antes da certificação)
 7. acompanhar            GET  /documents/{id}                    → status: certificated
                          ou webhook document_ready
 8. baixar o resultado    GET  /documents/{id}/download/certificated
```

```rust,no_run
use assinafy::Client;
use assinafy::models::{ArtifactName, AssignmentMethod, DocumentStatus};
use assinafy::resources::{CreateAssignmentBody, CreateSignerBody, UploadDocumentRequest};

async fn assinar_contrato() -> assinafy::Result<()> {
    let conta = std::env::var("ASSINAFY_ACCOUNT_ID").unwrap();
    let client = Client::builder()
        .api_key(std::env::var("ASSINAFY_API_KEY").unwrap())
        .build()?;

    // 1. O signatário precisa de e-mail ou WhatsApp para ser notificado.
    let signatario = client
        .signers(&conta)
        .create(&CreateSignerBody::new("Maria Silva").email("maria@example.invalid"))
        .await?;

    // 2. O upload é multipart; o documento nasce em `uploaded` e a extração de
    //    metadados ocorre de forma assíncrona.
    let arquivo = UploadDocumentRequest::from_path("./contrato.pdf").await?;
    let documento = client.documents().upload(&conta, arquivo).await?;

    // 3. Só é possível pedir assinaturas a partir de `metadata_ready`.
    let mut ready = false;
    for _ in 0..30 {
        let current = client.documents().get(&documento.id).await?;
        if matches!(current.status, DocumentStatus::MetadataReady) {
            ready = true;
            break;
        }
        if !matches!(current.status, DocumentStatus::Uploading | DocumentStatus::Uploaded | DocumentStatus::MetadataProcessing) {
            return Err(assinafy::Error::UnexpectedResponse(format!(
                "document processing stopped: {}", current.status
            )));
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    if !ready {
        return Err(assinafy::Error::UnexpectedResponse("document metadata timed out".into()));
    }

    // 4–5. Peça as assinaturas. `Virtual` notifica remotamente; `Collect`
    //      coleta presencialmente no dispositivo do remetente.
    let pedido = CreateAssignmentBody::new(
        AssignmentMethod::Virtual,
        [signatario.id.as_str()],
    )
    .message("Segue o contrato para assinatura.");

    let custo = client
        .assignments()
        .estimate_cost(&documento.id, &pedido)
        .await?;
    println!("custo estimado: {} créditos", custo.total_credits);

    if !custo.has_sufficient_resources {
        return Err(assinafy::Error::Validation("insufficient resources for signing".into()));
    }

    let assignment = client.assignments().create(&documento.id, &pedido).await?;

    // Os links de assinatura dão acesso ao documento: entregue-os por canal
    // privado e nunca os registre em log.
    for link in &assignment.signing_urls {
        println!("link emitido para o signatário {}", link.signer_id);
    }

    // 7–8. Depois que todos assinarem, o documento fica `certificated`.
    let atual = client.documents().get(&documento.id).await?;
    if matches!(atual.status, DocumentStatus::Certificated) {
        let (pdf, tipo) = client
            .documents()
            .download_artifact(&documento.id, ArtifactName::Certificated)
            .await?;
        assert_eq!(tipo, "application/pdf");
        tokio::fs::write("./contrato-certificado.pdf", pdf).await?;
    }

    Ok(())
}
```

O remetente recebe os links; o signatário conclui separadamente o fluxo de assinatura descrito
adiante. Trate a conclusão pelo webhook `document_ready` ou por consultas com prazo definido e
então baixe o artefato final. Guarde os IDs de documento e signatário para que uma retomada continue
a operação existente, sem repetir o upload ou o envio.

Estados do documento, em ordem de ciclo de vida:

| Status | Significado |
| --- | --- |
| `uploading` / `uploaded` | Upload em andamento / concluído |
| `metadata_processing` / `metadata_ready` | Extração de metadados / pronto para assignment |
| `pending_signature` | Aguardando um ou mais signatários |
| `pending_identity_review` | Todos assinaram; aguardando análise de identidade pela conta (`status.as_str()`) |
| `certificating` / `certificated` | Gerando o certificado / assinado e certificado |
| `rejected_by_signer` / `rejected_by_user` | Recusado pelo signatário / cancelado pelo remetente |
| `expired` | Prazo de assinatura vencido |
| `failed` | Processamento falhou |

Valores futuros que o SDK ainda não modela chegam como `DocumentStatus::Unknown(String)` em vez
de falharem na desserialização.

## Métodos de verificação do signatário

Definidos por signatário ao criar o assignment. O método de verificação e o de notificação são
**acoplados**: envie um, os dois ou nenhum — o lado que faltar é inferido. Sem nenhum dos dois,
ambos assumem `Email`.

| Método | Como funciona | Custo por signatário |
| --- | --- | --- |
| `Email` *(padrão)* | Código de uso único (OTP) por e-mail, exigido antes de assinar | Gratuito |
| `Whatsapp` | Código de uso único (OTP) por WhatsApp | Verificação gratuita; notificação 0,45 crédito, só em planos pagos |
| `DigitalCertificate` | O signatário assina com o **próprio certificado ICP-Brasil (A1/A3)**, pela extensão de navegador Web PKI, gerando uma assinatura **PAdES qualificada** | 0,5 crédito, somado ao custo da notificação |
| `IdDocumentFaceMatchBasic` | Captura hospedada do documento de identidade e da face, com consentimento biométrico explícito e análise pela conta quando necessária | Consulte `estimate_cost` para o preço atual da conta |

Combinações permitidas: `Email` → notifica por `Email`; `Whatsapp` → notifica por `Whatsapp`;
`DigitalCertificate` e `IdDocumentFaceMatchBasic` → notificam por `Email` **ou** `Whatsapp`. Apenas um método de notificação
por signatário.

```rust,no_run
use assinafy::Client;
use assinafy::models::{AssignmentMethod, NotificationMethod, VerificationMethod};
use assinafy::resources::{CreateAssignmentBody, CreateAssignmentSigner};

async fn pedir_com_metodos(documento: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("chave-de-api")?;

    let pedido = CreateAssignmentBody::from_signers(
        AssignmentMethod::Virtual,
        [
            // OTP por e-mail (padrão explícito), assina primeiro.
            CreateAssignmentSigner::new("sig_email")
                .step(1)
                .verification_method(VerificationMethod::Email),
            // OTP por WhatsApp: verificação e notificação viajam juntas.
            CreateAssignmentSigner::new("sig_whatsapp")
                .step(2)
                .verification_method(VerificationMethod::Whatsapp)
                .notification_methods(vec![NotificationMethod::Whatsapp]),
            // Certificado ICP-Brasil, notificado por e-mail.
            CreateAssignmentSigner::new("sig_certificado")
                .step(3)
                .verification_method(VerificationMethod::DigitalCertificate)
                .notification_methods(vec![NotificationMethod::Email]),
            // Captura biométrica, com notificação por email.
            CreateAssignmentSigner::new("sig_biometric")
                .step(4)
                .verification_method(VerificationMethod::id_document_face_match_basic())
                .notification_methods(vec![NotificationMethod::Email]),
        ],
    )
    .expires_at("2026-12-31T23:59:59Z");

    client.assignments().create(documento, &pedido).await?;
    Ok(())
}
```

O `step` define assinatura sequencial: o signatário do passo 2 só é notificado quando o passo 1
termina. Sem `step`, todos assinam em paralelo.

Estime antes de enviar — a estimativa informa saldo, custo total e o motivo de bloqueio quando
os créditos não bastam:

```rust,no_run
use assinafy::Client;
use assinafy::models::AssignmentMethod;
use assinafy::resources::EstimateAssignmentCostBody;

async fn estimar(documento: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("chave-de-api")?;
    let estimativa = client
        .assignments()
        .estimate_cost(
            documento,
            &EstimateAssignmentCostBody::new(AssignmentMethod::Virtual, ["sig_1", "sig_2"]),
        )
        .await?;

    println!("total: {} créditos", estimativa.total_credits);
    println!("saldo: {}", estimativa.credit_balance);
    for item in &estimativa.breakdown {
        println!("  {} × {} = {}", item.quantity, item.name, item.cost);
    }
    if let Some(motivo) = &estimativa.blocking_reason {
        let detalhe = estimativa.message.as_deref().unwrap_or("");
        println!("bloqueado: {motivo:?} — {detalhe}");
    }
    Ok(())
}
```

## Verificação biométrica de identidade

Use `VerificationMethod::id_document_face_match_basic()` nos signatários de assignments ou templates.
O método envia `IdDocumentFaceMatchBasic`, representado por `VerificationMethod::Other` na leitura.
`SignerSelf::verification_method` identifica esse fluxo; o signatário usa a captura hospedada do
documento e da face em vez do endpoint de OTP. Inicie somente após consentimento explícito:

```rust,no_run
use assinafy::Client;
use assinafy::models::IdentityVerificationSession;

async fn iniciar_captura(
    signatario: &Client,
    consentiu: bool,
) -> assinafy::Result<IdentityVerificationSession> {
    signatario.signer_self().start_identity_verification(consentiu).await
}
```

O cliente acima deve usar `Auth::AccessCode`. Abra `session.url` sem alterações, incluindo o
fragmento `#t=`. É um link de acesso: nunca registre em logs nem persista no armazenamento do
navegador. Consulte `signer_self().identity_verification()` com prazo máximo; inicie somente
quando `can_start` for verdadeiro (sem sessão ou `Expired`). `Pending` mantém a sessão existente.
`Declined` e `Abandoned` exigem novo documento e assignment. Reenviar a notificação não substitui
uma sessão ativa; uma captura expirada pode ser reiniciada sem nova cobrança de verificação.

Em `InReview`, o signatário pode confirmar os dados e assinar enquanto a conta analisa.
`AssignmentSigner::identity_verification_status` expõe esse estado. Depois de todas as assinaturas,
o documento aguarda em `pending_identity_review`; leia com `DocumentStatus::as_str()`.
A conta chama `assignments().identity_verification(documento, assignment, signatario)` para ler
motivos, verificações e o resumo das evidências. Os nomes em `images` podem ser passados a
`download_identity_image`; as imagens ficam indisponíveis após a decisão.

Após uma pessoa analisar as evidências, registre sua decisão explícita com o cliente da conta:

```rust,no_run
use assinafy::Client;
use assinafy::resources::{IdentityReviewBody, IdentityReviewDecision};

async fn registrar_analise(
    conta: &Client,
    documento: &str,
    assignment: &str,
    signatario: &str,
    decisao: IdentityReviewDecision,
) -> assinafy::Result<()> {
    conta.assignments().review_identity_verification(
        documento, assignment, signatario, &IdentityReviewBody::new(decisao),
    ).await?;
    Ok(())
}
```

`Approved` libera a certificação quando todas as assinaturas e análises terminarem. `Rejected`
cancela o documento e notifica todos os signatários e destinatários de cópia. A `.note(...)` é privada.

## Certificado digital ICP-Brasil (A1/A3)

Exige o recurso **Certificado Digital** na conta (planos Standard e Pro), CPF ou CNPJ em
`government_id` do signatário, e exatamente **um signatário por certificado naquele passo**. Um
CPF exige o certificado daquela pessoa (e-CPF, ou e-CNPJ que a nomeie como representante legal);
um CNPJ exige um e-CNPJ da empresa.

Antes de abrir o assignment, o signatário precisa confirmar os dados de identidade e aceitar os
termos. O endpoint comum de assinatura **rejeita** signatários por certificado — a assinatura
deles é produzida por um handshake de dois passos com a extensão Web PKI:

```text
POST /v1/signers/certificate/start     → data.token   (token da operação Web PKI)
        ↓  o navegador assina o token com o certificado do signatário
POST /v1/signers/certificate/complete  → data.signerName
```

> Essas duas rotas são extensões implantadas **somente em produção**: outras implantações não as expõem e
> elas não constam do documento OpenAPI publicado, portanto o SDK não as envolve.

Concluído o fluxo, baixar o artefato `pades` devolve a assinatura PAdES qualificada.

## Fluxo do signatário

Com `Auth::AccessCode`, o cliente passa a falar pelos endpoints do signatário. O código chega ao
signatário por e-mail ou WhatsApp e não pode ser obtido pela API. Esses métodos recusam um código
vazio e qualquer outra credencial, e enviam somente o código; para ler os documentos de um
signatário com a chave de API, use `list_documents_as_workspace` e `search_documents_as_workspace`.

```rust,no_run
use assinafy::models::{SignDocumentItem, SignerType};
use assinafy::resources::{ConfirmSignerDataBody, VerifyCodeBody};
use assinafy::{Auth, Client};

async fn assinar_como_signatario(documento: &str, assignment: &str) -> assinafy::Result<()> {
    let client = Client::builder()
        .auth(Auth::AccessCode("codigo-do-signatario".into()))
        .build()?;
    let signer = client.signer_self();

    // 1. Quem sou eu e o que devo assinar.
    let eu = signer.me().await?;
    let doc = signer.signable_document().await?;
    println!("{} vai assinar {}", eu.full_name, doc.name);

    // 2. Aceitar os termos e confirmar os dados de identidade.
    signer.accept_terms().await?;
    signer
        .confirm_data(
            documento,
            &ConfirmSignerDataBody::new()
                .full_name("Maria Silva")
                .government_id("123.456.789-09"),
        )
        .await?;

    // 3. Validar o código de uso único recebido por e-mail ou WhatsApp.
    signer.verify(&VerifyCodeBody::new("123456")).await?;

    // 4. Enviar a rubrica e preencher os campos do assignment.
    let png = tokio::fs::read("./assinatura.png").await?;
    signer
        .upload_signature(SignerType::Signature, "image/png", png)
        .await?;

    let itens: Vec<SignDocumentItem> = doc
        .assignment
        .iter()
        .flat_map(|a| a.items.iter())
        .filter(|item| !item.completed && item.signer.as_ref().is_some_and(|s| s.id == eu.id))
        .filter_map(|item| {
            let campo = item.field.as_ref()?;
            let pagina = item.page.as_ref()?;
            Some(SignDocumentItem::new(
                &item.id,
                &campo.id,
                &pagina.id,
                "Maria Silva",
            ))
        })
        .collect();
    signer.sign(documento, assignment, itens).await?;

    Ok(())
}
```

Recusar é igualmente explícito, e o motivo fica registrado no documento:

```rust,no_run
use assinafy::{Auth, Client};

async fn recusar(documento: &str, assignment: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("chave-de-api")?
        .with_auth(Auth::AccessCode("codigo-do-signatario".into()));
    client
        .signer_self()
        .decline(documento, assignment, "Dados incorretos no contrato")
        .await?;
    Ok(())
}
```

Um signatário com vários documentos pendentes pode assinar ou recusar em lote com
`sign_multiple` e `decline_multiple`.

## Templates

Um template é um documento reutilizável com papéis e campos já posicionados. Criar um documento
a partir dele dispensa upload e posicionamento.

```rust,no_run
use assinafy::Client;
use assinafy::models::VerificationMethod;
use assinafy::resources::{CreateDocumentFromTemplateBody, TemplateDocumentSigner};

async fn gerar_do_template(conta: &str, template: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("chave-de-api")?;

    // Cada papel do template recebe um signatário concreto.
    let corpo = CreateDocumentFromTemplateBody::default()
        .name("Contrato — Maria Silva")
        .message("Segue para assinatura.")
        .signers(vec![
            // Um signatário já cadastrado na conta…
            TemplateDocumentSigner::existing("role_contratante", "sig_1")
                .verification_method(VerificationMethod::Email),
            // …ou um criado na hora, a partir do nome e do contato.
            TemplateDocumentSigner::inline("role_contratado", "João Souza")
                .whatsapp("+5511999999999")
                .verification_method(VerificationMethod::Whatsapp),
        ]);

    let estimativa = client.templates(conta).estimate_cost(template, &corpo).await?;
    println!("custo: {} créditos", estimativa.total_credits);

    let documento = client.templates(conta).create_document(template, &corpo).await?;
    println!("documento {} criado e já em assinatura", documento.id);
    Ok(())
}
```

Listar, obter, criar (multipart), atualizar e excluir templates também estão disponíveis, assim
como o download de cada página renderizada.

## Tags

```rust,no_run
use assinafy::Client;
use assinafy::resources::CreateTagBody;

async fn etiquetar(conta: &str, documento: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("chave-de-api")?;
    let tags = client.tags(conta);

    let contratos = tags.create(&CreateTagBody::new("Contratos").color("3399ff")).await?;
    println!("tag {} criada", contratos.id);

    // Anexar e substituir recebem **nomes**; remover recebe o **id**.
    tags.add_to_document(documento, ["Contratos", "Urgente"]).await?;
    tags.set_on_document(documento, ["Assinado"]).await?;
    tags.remove_from_document(documento, &contratos.id).await?;
    Ok(())
}
```

`add_to_document` e `set_on_document` fazem *upsert por nome*: a API compara sem diferenciar
maiúsculas com as tags existentes da conta e cria a que faltar. Passar um id de tag aqui cria uma
tag nova com aquele id como nome. `remove_from_document` é a única operação de tag de documento
que recebe um id real, como devolvido por `create` ou `list_for_document`.

## Campos personalizados

Campos definem os dados que um signatário preenche. Cada conta tem os campos padrão da
plataforma mais os que você criar; a validação por expressão regular acontece no servidor.

```rust,no_run
use assinafy::Client;
use assinafy::resources::{CreateFieldBody, ValidateFieldEntry};

async fn campos(conta: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("chave-de-api")?;
    let fields = client.fields(conta);

    for tipo in fields.list_types().await? {
        println!("tipo disponível: {} ({})", tipo.name, tipo.kind);
    }

    // A regex precisa vir delimitada por barras — `^.{2,}$` sem barras é
    // recusada com `400 Padrão RegEx inválido.`
    let campo = fields
        .create(
            &CreateFieldBody::new("text", "Matrícula")
                .regex("/^[0-9]{6}$/")
                .required(true),
        )
        .await?;

    let resultado = fields.validate(&campo.id, "123456").await?;
    println!("válido: {}", resultado.success);

    // Ou valide vários de uma vez.
    let lote = fields
        .validate_multiple([ValidateFieldEntry::new(&campo.id, "123456")])
        .await?;
    println!("{} resultados", lote.len());
    Ok(())
}
```

## Webhooks

Cada conta tem um endpoint de webhook, ou até três nos planos pagos. Todo endpoint ativo inscrito
em um evento o recebe; a API guarda o histórico de entregas, que pode ser reenviado. Com
`signing_enabled`, cada entrega leva a assinatura [Standard Webhooks](https://www.standardwebhooks.com)
nos cabeçalhos `webhook-id`, `webhook-timestamp` e `webhook-signature`.

```rust,no_run
use assinafy::Client;
use assinafy::resources::WebhookEndpointBody;

async fn webhooks(conta: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("chave-de-api")?;
    let hooks = client.webhooks(conta);

    for tipo in hooks.event_types().await? {
        println!("{}: {}", tipo.id, tipo.description);
    }

    let endpoint = hooks
        .create_endpoint(
            &WebhookEndpointBody::new("https://app.example.invalid/hooks", "ops@example.invalid")
                .name("ERP")
                .events(["document_ready", "signer_rejected_document"])
                .signing_enabled(true),
        )
        .await?;
    // Guarde o segredo `whsec_…` no cofre de segredos do receptor.
    let segredo = endpoint.secret.as_deref().ok_or_else(||
        assinafy::Error::UnexpectedResponse("segredo de webhook ausente".into())
    )?;
    let _ = segredo;

    // Alterações enviam só os campos definidos.
    hooks
        .update_endpoint(&endpoint.id, &WebhookEndpointBody::default().active(false))
        .await?;

    // Histórico de entregas do endpoint, com reenvio das que falharam.
    let entregas = hooks
        .list_dispatches()
        .endpoint_id(&endpoint.id)
        .delivered(false)
        .per_page(50)
        .send()
        .await?;
    for entrega in &entregas.data {
        println!("{} → HTTP {:?}", entrega.event, entrega.http_status);
        hooks.retry_dispatch(&entrega.id).await?;
    }

    hooks.delete_endpoint(&endpoint.id).await?;
    Ok(())
}
```

`rotate_endpoint_secret` troca o segredo e invalida o anterior na hora. As rotas de segredo não
estão disponíveis para aplicativos OAuth; guarde `WebhookEndpoint::secret` da criação.
Se uma aplicação OAuth o perder, exclua e recrie esse endpoint. Use `list_endpoints_by_url` para
localizar a URL exata e `delete_endpoint_by_url` para removê-la. As rotas `register`,
`get_subscription` e `inactivate` continuam operando sobre o endpoint mais antigo da conta.

No receptor, valide o corpo **bruto**, exatamente como recebido, antes de interpretá-lo:

```rust
use assinafy::resources::verify_webhook_signature;

fn receber(
    segredo: &str,
    id: &str,
    timestamp: &str,
    assinatura: &str,
    corpo: &[u8],
) -> assinafy::Result<serde_json::Value> {
    verify_webhook_signature(segredo, id, timestamp, assinatura, corpo)?;
    Ok(serde_json::from_slice(corpo)?)
}
```

A verificação compara em tempo constante e rejeita timestamps a mais de
`WEBHOOK_TIMESTAMP_TOLERANCE` (cinco minutos) do relógio local. Use o cabeçalho `webhook-id` para
descartar entregas repetidas.

## Atividades e artefatos

`Client::activities` devolve todos os eventos registrados de um documento, cada um com um
snapshot do `payload` do evento e a `origin` da requisição (`ip`, `user-agent`).

```rust,no_run
use assinafy::Client;

async fn trilha(documento: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("chave-de-api")?;
    let eventos = client.activities().list(documento).per_page(100).send().await?;
    for evento in &eventos.data {
        let ip = evento.origin.as_ref().and_then(|o| o.ip.as_deref()).unwrap_or("-");
        let mensagem = evento.message.as_deref().unwrap_or("");
        println!("{} — {mensagem} (de {ip})", evento.event);
    }
    Ok(())
}
```

Artefatos disponíveis para download:

| Artefato | Conteúdo |
| --- | --- |
| `original` | O PDF enviado, como recebido |
| `certificated` | O documento assinado, com a certificação da plataforma |
| `certificate-page` | Apenas a página de certificação |
| `pades` | Assinaturas ICP-Brasil dos signatários + caixa de certificação — só existe em documentos que tiveram signatários por certificado digital |
| `bundle` | Zip com `original`, `certificated` e `certificate-page`, mais o `pades` quando houver |

`thumbnail` não é um valor válido na rota `download/{artefato}`; o SDK redireciona
`ArtifactName::Thumbnail` para `GET /documents/{id}/thumbnail` de forma transparente.

A verificação pública confere um documento assinado pelo hash da assinatura, sem autenticação:

```rust,no_run
use assinafy::Client;

async fn verificar(hash: &str) -> assinafy::Result<()> {
    let client = Client::builder().build()?;
    let resultado = client.documents().verify(hash).await?;
    println!("válido: {} — {}", resultado.is_valid, resultado.message);
    Ok(())
}
```

## Paginação

Todo endpoint paginado devolve `Page<T>` com `data` e `meta`, esta lida dos cabeçalhos
`X-Pagination-*`. `meta` também traz o estado do limite de taxa (`X-Rate-Limit-*`), útil para se
autorregular antes de levar um `429`.

```rust,no_run
use assinafy::Client;

async fn percorrer(conta: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("chave-de-api")?;
    let mut pagina = Some(1);
    while let Some(n) = pagina {
        let resultado = client.signers(conta).list().page(n).per_page(100).send().await?;
        println!("página {n}: {} itens", resultado.data.len());
        if let Some(restantes) = resultado.meta.rate_limit_remaining {
            println!("  {restantes} requisições restantes na janela");
        }
        pagina = resultado.next_page();
    }
    Ok(())
}
```

## Erros e limites de taxa

Toda operação devolve `Result<T, Error>`. Erros da API preservam o status HTTP, a mensagem do
servidor e o payload `data` bruto, então dá para ramificar por código sem perder informação.

```rust,no_run
use assinafy::{Client, Error};

async fn tratar(conta: &str) -> assinafy::Result<()> {
    let client = Client::from_api_key("chave-de-api")?;
    match client.signers(conta).get("inexistente").await {
        Ok(signatario) => println!("{}", signatario.full_name),
        Err(e) if e.is_rate_limited() => {
            println!("aguarde {:?}s antes de tentar de novo", e.retry_after());
        }
        Err(Error::Api(e)) if e.status == 404 => println!("não encontrado: {}", e.message),
        Err(Error::Api(e)) => println!("erro {}: {} — {:?}", e.status, e.message, e.data),
        Err(outro) => return Err(outro),
    }
    Ok(())
}
```

`e.rate_limit()` traz a janela `X-Rate-Limit-Limit`/`-Remaining`/`-Reset` informada no erro;
`retry_after()` lê o `Retry-After` enviado com o `429`.

Variantes de `Error`: `Config` (configuração inválida), `Http` (transporte), `Serde`, `Io`,
`Url`, `Api` (resposta não-2xx), `UnexpectedResponse` (payload indecifrável) e `Validation`
(requisição recusada localmente, antes do envio). Credenciais nunca
aparecem em `Display`/`Debug` — a URL é removida dos erros de transporte, e tokens em `Debug`
saem como `**redacted**`.

## Índice de operações

| Recurso | Operações |
| --- | --- |
| Autenticação | `login`, `verify_mfa`, `social_login`, `change_password`, `request_password_reset`, `reset_password`, `link_social_login`, `social_login_url` |
| OAuth 2.1 | `protected_resource_metadata`, `authorization_server_metadata`, `token`, `revoke`, `userinfo` |
| Contas | `list`, `create`, `get`, `update`, `delete`, `delete_forcing`, `theme`, `stats`, `download_logo`, `upload_logo`, `delete_logo` |
| Chaves de API | `create`, `get`, `delete` |
| Usuários | `me`, `stats`, `notification_preferences`, `update_notification_preferences`, `mfa_methods`, `start_totp_enrollment`, `confirm_totp_enrollment`, `regenerate_recovery_codes`, `remove_mfa_method` |
| Signatários | `create`, `list`, `get`, `update`, `delete` |
| Documentos | `statuses`, `list`, `upload`, `search`, `get`, `rename`, `delete`, `download_artifact`, `download_thumbnail`, `download_page`, `verify` |
| Assignments | `list`, `list_current`, `create`, `estimate_cost`, `reset_expiration`, `reset_expiration_at`, `resend_to_signer`, `estimate_resend_cost`, `whatsapp_notifications`, `identity_verification`, `download_identity_image`, `review_identity_verification`, `sign`, `reject` |
| Tags | `list`, `create`, `update`, `delete`, `delete_with_force`, `list_for_document`, `add_to_document`, `set_on_document`, `remove_from_document` |
| Campos | `create`, `list`, `get`, `update`, `delete`, `validate`, `validate_multiple`, `list_types` |
| Templates | `list`, `get`, `create`, `update`, `delete`, `download_page`, `create_document`, `estimate_cost` |
| Webhooks | `list_endpoints`, `list_endpoints_by_url`, `delete_endpoint_by_url`, `create_endpoint`, `get_endpoint`, `update_endpoint`, `delete_endpoint`, `endpoint_secret`, `rotate_endpoint_secret`, `register`, `get_subscription`, `inactivate`, `event_types`, `list_dispatches`, `retry_dispatch`; `verify_webhook_signature` |
| Atividades | `list` |
| Públicos | `document`, `send_token`, `send_token_with_result` |
| Signatário (self) | `me`, `identity_verification`, `start_identity_verification`, `accept_terms`, `verify`, `confirm_data`, `signable_document`, `signable_document_with_accepted_terms`, `current_document`, `list_documents`, `list_documents_as_workspace`, `search_documents`, `search_documents_as_workspace`, `sign`, `decline`, `sign_multiple`, `decline_multiple`, `download_document`, `upload_signature`, `upload_signature_with_reuse`, `download_signature` |

Cada método traz no Rustdoc a rota HTTP que chama e os payloads de requisição e resposta.
Exemplos executáveis ficam em [`examples/`](examples).

## Features do Cargo

* `rustls-tls` *(padrão)* — TLS via [rustls](https://docs.rs/rustls).
* `native-tls` — TLS pela pilha nativa do sistema operacional.

## Testes

```bash
cargo test --all-targets --all-features --locked
cargo test --doc --all-features --locked
```

Os testes são offline: formatos de requisição, validação, valores desconhecidos, paginação,
redação de credenciais, roteamento seguro de downloads e verificação de assinaturas de webhook.

### Conexão OAuth interativa

O teste interativo de conexão em produção verifica autorização, acesso ao workspace, userinfo,
rotação do refresh token e revogação. Cadastre um aplicativo temporário com `documents:read`,
`account:read`, `openid` e `offline_access` e abra um túnel HTTPS local:

```bash
cloudflared tunnel --url http://127.0.0.1:62477 --no-autoupdate
```

Cadastre a URL HTTPS do túnel com `/callback` como URI exata de retorno. Em outro terminal, defina
os valores de execução abaixo e abra em até cinco minutos a URL de autorização exibida pelo teste:

```bash
export ASSINAFY_OAUTH_CLIENT_ID='registered-client-id'
export ASSINAFY_OAUTH_REDIRECT_URI='https://your-tunnel.example.invalid/callback'
export ASSINAFY_OAUTH_CALLBACK_ADDR='127.0.0.1:62477'
# Confidential apps also set ASSINAFY_OAUTH_CLIENT_SECRET.
cargo test --test oauth --locked -- --ignored --nocapture
```

O teste revoga os tokens antes de validar o resultado. Encerre o túnel ao terminar. A assinatura por
OTP de email depende do código recebido pelo destinatário; a assinatura A1/A3 depende do certificado
do próprio signatário e da extensão Web PKI. Essas etapas interativas são separadas dos testes com
chave de API. A notificação por WhatsApp exige um plano elegível.

## Licença

Distribuído sob a licença [MIT](LICENSE).
