# FinTrack · Carteira de Investimentos

**Criador desta versão:** Ronaldo Melo

Aplicação fullstack em Rust para cadastrar e consultar ativos de investimento. A interface web oferece cadastro e login, com sessão mantida por cookie e JWT. A API lista, cria e atualiza ativos.

Projeto desenvolvido a partir do [repositório-base do desafio da DIO](https://github.com/digitalinnovationone/rust-fullstack-carteira-investimentos).

## Tecnologias e estrutura MVC

- **Model** — `src/models.rs` define ativos e usuários. `src/repository.rs` concentra consultas e transações Oracle. `src/db.rs` configura o pool de conexões e executa o driver síncrono fora das threads assíncronas do Tokio.
- **View** — `templates/` contém os templates HTML renderizados pelo Askama.
- **Controller** — `src/routes/` contém os handlers Axum da API e da interface web. `src/auth/` implementa a autenticação com cookies, JWT e hash de senha.
- **Inicialização** — `src/app.rs` monta a aplicação. `migrations/` contém o SQL versionado do Oracle Database.

A melhoria do desafio foi trocar PostgreSQL e SQLx pelo Oracle Database Free 26ai do Docker, usando o driver [`oracledb`](https://github.com/oracle/rust-oracledb). O driver dispensa a instalação do Oracle Instant Client.

## Como executar

Use Rust com Cargo e um Oracle Database Free 26ai em execução. A configuração de exemplo usa o contêiner `oracle26ai`, a porta publicada `1521` e o serviço `FREEPDB1`.

```powershell
docker ps --filter name=oracle26ai
if (-not (Test-Path .env)) { Copy-Item .env.example .env }
```

Se o schema `fintrack` ainda não existir, entre no SQL*Plus com acesso administrativo local.

```powershell
docker exec -it oracle26ai sqlplus / as sysdba
```

No SQL*Plus, troque a senha de exemplo antes de executar.

```sql
ALTER SESSION SET CONTAINER = FREEPDB1;
CREATE USER fintrack IDENTIFIED BY "substitua_pela_sua_senha"
    DEFAULT TABLESPACE USERS QUOTA 50M ON USERS;
GRANT CREATE SESSION, CREATE TABLE, CREATE SEQUENCE TO fintrack;
EXIT
```

Preencha o `.env` com a senha do schema. Para aplicar a migração, copie o SQL para o contêiner e entre como o usuário da aplicação. O SQL*Plus solicitará a senha.

```powershell
docker cp migrations/001_create_tables.sql oracle26ai:/tmp/fintrack-schema.sql
docker exec -it oracle26ai sqlplus fintrack@//localhost:1521/FREEPDB1
```

```sql
@/tmp/fintrack-schema.sql
EXIT
```

A migração pode ser reaplicada. Em seguida, execute `cargo run --locked` e abra [a página de login](http://localhost:3000/login). O primeiro login com um nome novo cria a conta. Na máquina em que o schema já foi provisionado, basta usar o `.env` local e iniciar a aplicação.

## Como testar

Com o Oracle ativo, as credenciais no `.env` e a migração aplicada, execute:

```powershell
cargo test --locked -- --include-ignored
```

O teste de integração cria dados com nomes exclusivos, verifica cadastro, listagem e atualização de ativos, cadastro e autenticação de usuários e tratamento de usuário duplicado. Ao terminar, ele limpa os registros que criou.

Para testar pela API com a aplicação em execução:

```powershell
$base = 'http://localhost:3000/api/assets'
$headers = @{ Authorization = 'im-the-admin' }
$asset = Invoke-RestMethod $base -Method Post -Headers $headers -ContentType 'application/json' -Body (@{
    name = "Ativo de teste $([guid]::NewGuid())"
    unit_value = 10.5
} | ConvertTo-Json)
Invoke-RestMethod $base
Invoke-RestMethod $base -Method Patch -Headers $headers -ContentType 'application/json' -Body (@{
    id = $asset.id
    unit_value = 20.5
} | ConvertTo-Json)
```

A chave de administração acima é a credencial de demonstração do projeto-base, usada nos testes locais.

## O que aprendi

A adaptação mostrou como separar a persistência dos controladores e da interface, parametrizar consultas Oracle, gerar IDs com sequences e controlar transações explícitas. Também mostrou como integrar um driver síncrono a uma aplicação assíncrona sem bloquear as threads que atendem às requisições.
