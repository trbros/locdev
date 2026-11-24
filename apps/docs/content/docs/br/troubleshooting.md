# Solução de problemas

Este guia ajuda a escolher a próxima ação a partir de uma falha observada. Para entender cada verificação, consulte a [referência do Doctor](/docs/doctor). Os comandos abaixo usam `acme` como exemplo: substitua pelo nome de um projeto registrado. Nenhum comando é executado pelo site.

## Comece pelas evidências

```bash
locdev version
locdev status
locdev doctor
locdev list
```

Anote a versão, sistema operacional, arquitetura e a primeira verificação que falhou. Não reinstale o ambiente inteiro antes de identificar qual camada apresenta o problema.

## O daemon não responde

```bash
locdev start
locdev status
```

Se continuar indisponível, confira se `locdev` e `locdevd` pertencem à mesma instalação e consulte os logs do daemon no diretório de dados descrito em [Configuração](/docs/configuration). Para versões incompatíveis, siga [Atualização](/docs/updating). Iniciar o daemon também pode iniciar projetos com `auto_start` habilitado.

## O projeto falha ao iniciar

```bash
locdev logs acme
locdev domain acme
```

Confira o diretório registrado, o comando efetivo, o runtime e as dependências do projeto. Instale dependências pelo gerenciador e lockfile corretos, fora do site. Depois de corrigir a causa, reinicie apenas esse projeto:

```bash
locdev reload acme
```

## Portas e proxy

O Doctor diferencia a porta do servidor do projeto das portas do proxy. Se 80/443 estiverem ocupadas, a configuração padrão permite fallback para 7080/7443. Use a porta efetivamente indicada pelo app; não presuma que a URL sem porta aponta para o LocDev.

Revise [Configuração](/docs/configuration) antes de mudar portas. Encerre outro servidor somente se você souber a quem ele pertence; não interrompa processos desconhecidos nem aplique privilégios automaticamente.

## O domínio .test não resolve

```bash
locdev dns status
```

Se outra ferramenta controlar o resolvedor, revise a origem do conflito antes de alterar a configuração do sistema. Somente quando quiser direcionar `.test` ao LocDev, execute e revise o plano solicitado:

```bash
locdev dns install
```

Consulte [Domínios](/docs/domains) para `.localhost` e detalhes por sistema.

## HTTPS mostra aviso de certificado

```bash
locdev tls status
```

Confira a CA local e o armazenamento de certificados do navegador usado. Se a CA correta ainda não for confiável, revise as instruções de [HTTPS](/docs/https) e a alteração proposta por:

```bash
locdev tls trust
```

Reabra o navegador após a alteração. Não ignore um certificado diferente do esperado.

## O servidor de desenvolvimento rejeita o host

Nos processos que o LocDev inicia, o Vite já recebe os nomes do site pela variável `__VITE_ADDITIONAL_SERVER_ALLOWED_HOSTS` (veja [Servidores de desenvolvimento](/docs/servers)). Se o Vite roda por fora do LocDev, inclua o domínio específico que você controla, como `acme.test`, em `server.allowedHosts`. Não use `true` para aceitar qualquer host. Consulte as [opções oficiais do Vite](https://vite.dev/config/server-options.html#server-allowedhosts) e adapte a configuração ao framework instalado.

## Quando pedir ajuda

Use a [central de suporte](/support) com os passos de reprodução e logs sem senhas, tokens, arquivos `.env` ou dados pessoais. Para uma possível vulnerabilidade, use o [canal privado](/security), não uma issue pública.
