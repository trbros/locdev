# Tutorial: versões de PHP e Node.js por projeto e no terminal

Cada projeto pode rodar com a sua versão, e o terminal pode usar a versão padrão que você escolher no app. Neste tutorial você instala duas versões, fixa uma num site e passa a usar a padrão no terminal.

## 1. Instale as versões

```bash
locdev runtime available php        # a mais nova de cada linha
locdev runtime install php 8.4
locdev runtime install php 8.3
locdev runtime install node lts
```

No app, **Runtimes → PHP** (ou **Node.js**) **→ Instalar versão…**. Versões já instaladas na máquina (pacotes do sistema, Homebrew, nvm, fnm) aparecem também e podem ser usadas; o LocDev nunca as altera.

## 2. Fixe uma versão num site

```bash
locdev edit loja --php 8.3
locdev reload loja
locdev info loja                     # php 8.3.x (projects.toml 8.3)
```

Um pin pode ser a linha (`8.3`, que segue a versão mais nova instalada dela) ou uma versão exata (`8.3.37`). Sem pin, o LocDev usa o arquivo de versão do projeto (`.php-version`, `.nvmrc`…), depois a faixa que o projeto declara (`require.php`, `engines.node`) e, por fim, a versão padrão.

## 3. Escolha a versão padrão

```bash
locdev runtime default php 8.4
locdev runtime default node 22
```

A padrão vale para os sites sem pin. No app, ela fica no menu **⋯** de cada versão.

## 4. Use a padrão no terminal

Ligue **Usar as versões padrão no terminal** em **Runtimes → Visão geral**, ou:

```bash
locdev runtime terminal on
```

![Runtimes: as versões padrão e o terminal.](screenshot:runtimes-terminal)

Abra um terminal novo: `php -v`, `node -v`, `npm`, `composer` passam a ser as versões padrão. Trocar a padrão no app troca o que o terminal usa, sem reabrir nada. Para voltar ao que era antes:

```bash
locdev runtime terminal off
```

No Linux e no macOS o LocDev acrescenta um bloco marcado ao `~/.bashrc` e ao `~/.zshrc` (e um arquivo próprio no fish); desligar remove só esse bloco. No Windows, a pasta vai para o início do `Path` do usuário.

## 5. Atualize e remova com segurança

```bash
locdev runtime update php 8.4        # instala o patch mais novo da linha
locdev runtime uninstall php 8.3.37
```

A versão antiga continua instalada depois de atualizar, para não quebrar sites que fixaram a versão exata. Remover é recusado enquanto um site em execução usa a versão.
