# assinafy — SDK Rust

*Português · [Read in English](README.en.md)*

[![Crate](https://img.shields.io/crates/v/assinafy.svg)](https://crates.io/crates/assinafy)
[![Docs](https://docs.rs/assinafy/badge.svg)](https://docs.rs/assinafy)

Cliente Rust assíncrono e idiomático para a API de assinatura eletrônica da
[Assinafy](https://assinafy.com.br) — plataforma brasileira de assinatura de documentos.

> **Referência completa em inglês.** Este documento cobre instalação, autenticação e os fluxos
> principais. O guia completo está em **[README.en.md](README.en.md)**, e a documentação da API em
> [docs.rs/assinafy](https://docs.rs/assinafy).

O SDK cobre a superfície REST pública documentada em <https://api.assinafy.com.br/v1/docs>:

| Área | Módulo |
| ---- | ------ |
| Autenticação | `Client::auth_api` |
| Contas | `Client::account` / `Client::accounts_api` |
| Chaves de API | `Client::api_keys` |
| Signatários | `Client::signers` |
| Signatário (self) | `Client::signer_self` |
| Documentos | `Client::documents` |
| Assignments | `Client::assignments` |
| Tags | `Client::tags` |
| Campos | `Client::fields` |
| Templates | `Client::templates` |
| Webhooks | `Client::webhooks` |
| Atividades | `Client::activities` |
| Usuários | `Client::users` |
| Endpoints públicos | `Client::public` |

## Instalação

Requer Rust 1.86 ou superior e usa a edição Rust 2024. A CI testa tanto o mínimo declarado (1.86)
quanto o stable atual — Rust não tem canal LTS.

```toml
[dependencies]
assinafy = "2"
tokio    = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Início rápido

```rust
use assinafy::Client;

let client = Client::builder()
    .api_key(std::env::var("ASSINAFY_API_KEY").unwrap())
    .sandbox() // omita para produção
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
```

## Autenticação

```rust
use assinafy::{Auth, Client};

// Servidor-a-servidor por chave de API (padrão para a maioria dos casos).
let c1 = Client::builder().api_key("chave-de-api").build().unwrap();

// Fluxo por token de usuário.
let c2 = Client::builder()
    .bearer("token-de-acesso")
    .build()
    .unwrap();

// Fluxo por token de acesso em query string, quando a integração exigir.
let c2_query = Client::builder()
    .access_token("token-de-acesso")
    .build()
    .unwrap();

// Endpoints do signatário usam o código de acesso da URL.
let c3 = c1.with_auth(Auth::AccessCode("token-do-signatario".into()));
```

## Métodos de verificação do signatário

Definidos por signatário ao criar o assignment. O método de verificação e o de notificação são
**acoplados**: envie um, os dois ou nenhum — o lado que faltar é inferido. Sem nenhum dos dois, ambos
assumem `Email`.

| Método | Como funciona | Custo por signatário |
| --- | --- | --- |
| `Email` *(padrão)* | Código de uso único (OTP) por e-mail, exigido antes de assinar | Gratuito |
| `Whatsapp` | Código de uso único (OTP) por WhatsApp | Verificação gratuita; notificação 0,45 crédito, só em planos pagos |
| `DigitalCertificate` | O signatário assina com o **próprio certificado ICP-Brasil (A1/A3)**, pela extensão de navegador Web PKI, gerando uma assinatura **PAdES qualificada** | 2 créditos |

Combinações permitidas: `Email` → notifica por `Email`; `Whatsapp` → notifica por `Whatsapp`;
`DigitalCertificate` → notifica por `Email` **ou** `Whatsapp`. Apenas um método de notificação por
signatário.

### Certificado digital ICP-Brasil

Exige o recurso **Certificado Digital** na conta (planos Standard e Pro), CPF ou CNPJ em
`government_id` do signatário, e exatamente **um signatário por certificado naquele passo**. Um CPF
exige o certificado daquela pessoa (e-CPF, ou e-CNPJ que a nomeie como representante legal); um CNPJ
exige um e-CNPJ da empresa.

Estime o custo antes: a assinatura por certificado custa 2 créditos por signatário, além do custo da
notificação escolhida.

Antes de abrir o assignment, o signatário precisa confirmar os dados de identidade e aceitar os
termos. O endpoint comum de assinatura **rejeita** signatários por certificado — a assinatura deles é
produzida por um handshake de dois passos com a extensão Web PKI:

```
POST /v1/signers/certificate/start     → data.token   (token da operação Web PKI)
        ↓  o navegador assina o token com o certificado do signatário
POST /v1/signers/certificate/complete  → data.signerName
```

> Essas duas rotas são extensões implantadas **somente em produção**: o sandbox não as expõe e elas
> não constam do documento OpenAPI publicado.

Concluído o fluxo, baixar o artefato `pades` devolve a assinatura PAdES qualificada.

## Trilha de atividades e artefatos

`Client::activities` devolve todos os eventos registrados de um documento, cada um com um snapshot do
`payload` do evento e a `origin` da requisição (`ip`, `user-agent`).

Artefatos disponíveis para download:

| Artefato | Conteúdo |
| --- | --- |
| `original` | O PDF enviado, como recebido |
| `certificated` | O documento assinado, com a certificação da plataforma |
| `certificate-page` | Apenas a página de certificação |
| `pades` | Assinaturas ICP-Brasil dos signatários + caixa de certificação — só existe em documentos que tiveram signatários por certificado digital |
| `bundle` | Zip com `original`, `certificated` e `certificate-page`, mais o `pades` quando houver |

A verificação pública confere um documento assinado pelo hash da assinatura, sem autenticação.

## Ambientes

| | |
| --- | --- |
| Produção | `https://api.assinafy.com.br/v1` (padrão) |
| Sandbox | `.sandbox()` no builder |

O sandbox é gratuito e espelha a produção para testar a integração de ponta a ponta — com a exceção
das rotas de certificado digital, que existem apenas em produção.

## Documentação

- **[README.en.md](README.en.md)** — guia completo, em inglês
- [docs.rs/assinafy](https://docs.rs/assinafy) — referência da API
- [Documentação da API](https://api.assinafy.com.br/v1/docs)

## Licença

Distribuído sob a licença [MIT](LICENSE).
