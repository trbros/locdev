# Segurança

O LocDev mexe em DNS, certificados e processos, então foi desenhado para precisar de pouca confiança.

## Privilégios

- O daemon, a CLI e o app rodam como o seu usuário.
- Mudanças no sistema (resolvedor, trust store) só acontecem com um script **visível** e **confirmado**, executado pelo mecanismo de autorização do SO.
- Tudo o que o LocDev escreve no sistema leva a marca `Managed by LocDev`, e os comandos de desinstalação só removem o que tem essa marca.
- Configurações de outras ferramentas são detectadas e mostradas, nunca editadas.

## Rede

- Proxy, HTTPS e DNS escutam apenas em `127.0.0.1` (e `::1`), nunca na rede.
- O servidor DNS responde só pela zona `.test`, recusa as demais e ignora pacotes que já são respostas, evitando loops de reflexão.
- O IPC usa um socket com permissão `0600` em um diretório `0700` do próprio usuário (Unix) ou um named pipe que rejeita clientes remotos (Windows).

## Certificados

- A chave da CA fica em `0600` na pasta de dados do usuário.
- A CA tem **Name Constraints** (`test`, `localhost`) e `pathlen:0`: ela não consegue emitir certificados válidos para outros domínios nem CAs intermediárias.
- Confiar na CA é opcional; sem isso, o HTTPS funciona, mas os navegadores avisam.

## Processos

- Os sites rodam como o seu usuário, no diretório do projeto.
- Parar um site encerra a árvore de processos inteira, para não deixar órfãos.
- Remover um site nunca apaga arquivos.

## Git e GitHub

- Operações destrutivas no Git (reset, descartar, _force push_…) mostram o que será perdido e exigem uma frase digitada. Opcionalmente, também pedem uma senha (hash Argon2id, bloqueio progressivo). Antes de rodar, o LocDev guarda um backup. A senha protege só o que é iniciado pelo LocDev, não o repositório. Veja [Git, GitHub e proteção](/docs/git).
- O token do GitHub fica só no keyring do sistema, nunca em arquivo ou log. O daemon só se comunica com `api.github.com`, e toda ação remota pede confirmação.
- O `git` roda sem shell, com argumentos separados e nomes validados, e nunca pede credenciais interativamente.

## Secrets e credenciais

- Variáveis secretas ficam cifradas (ChaCha20-Poly1305) com a chave no keyring do sistema, e só entram no ambiente dos processos, nunca em arquivos, argumentos ou logs ([Variáveis de ambiente](/docs/environment)).
- As credenciais de banco são lidas do projeto pelo daemon a cada uso e não passam pelo IPC; banco remoto só abre com uma escolha explícita, e SQL destrutivo exige o nome do banco ([Database Manager](/docs/database)).

## E-mail

- O Mailbox só captura, em loopback, e nunca entrega mensagens. Trocar um SMTP real do projeto exige o seu consentimento ([Mailbox](/docs/mailbox)).
- O HTML dos e-mails é mostrado num `iframe` com `sandbox` vazio e uma política que bloqueia toda a rede.
- O código do projeto só roda quando você pede (preview pelo framework, `make:mail`), num processo separado com ambiente limpo, tempo e saída limitados ([Email Studio](/docs/email-studio)).

## Reportando vulnerabilidades

Por favor, reporte problemas de segurança de forma privada aos mantenedores, antes de divulgá-los publicamente.
