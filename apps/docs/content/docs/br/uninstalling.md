# Desinstalar o LocDev

Escolha o que deseja remover: um projeto registrado, o início automático ou a instalação inteira. Desinstalar o app não deve ser confundido com apagar o código dos seus projetos. Faça backup antes de alterar dados locais.

## Remover apenas um projeto

Com `acme` substituído pelo nome registrado:

```bash
locdev down acme
locdev remove acme
```

`remove` desfaz o registro; os arquivos do projeto não são apagados (`--delete-files` move a pasta para a lixeira). Se esse era seu objetivo, não é necessário desinstalar o app nem remover DNS e certificados usados por outros projetos.

## Preparar a remoção completa

Mantenha a CLI disponível e o daemon em execução durante a preparação: os comandos de DNS e TLS consultam seu estado. Feche o desktop para que ele não reabra o daemon mais tarde. Pare os projetos em uso com `locdev down` e seus respectivos nomes.

Se o início automático estiver habilitado, remova-o antes de apagar os executáveis:

```bash
locdev startup status
locdev startup uninstall
```

Isso remove o mecanismo de login; não equivale a desinstalar os binários. Consulte [Início automático](/docs/service) para os detalhes por sistema.

## Reverter DNS e confiança TLS

Essas alterações afetam o ambiente LocDev do usuário, incluindo outros projetos locais. Execute somente se quiser retirar essa integração do sistema. Revise os planos e os pedidos de autorização; não use confirmação automática.

Com o daemon ainda disponível:

```bash
locdev dns uninstall
locdev tls untrust
```

A primeira operação desfaz a integração de resolução `.test`; a segunda remove a confiança na CA local dos armazenamentos tratados pelo app. Não apagam pastas de projetos. Se alguma falhar, confira o estado e use [Solução de problemas](/docs/troubleshooting) antes de continuar.

## Parar e remover os binários

Depois da preparação:

```bash
locdev stop
```

Esse comando encerra o daemon e os projetos restantes. Use o desinstalador ou gerenciador de pacotes correspondente à instalação original. Para uma CLI extraída manualmente, remova apenas os binários `locdev` e `locdevd` da pasta escolhida e revise a entrada do `PATH`.

## Decidir sobre os dados restantes

Configuração, registro de projetos, logs, runtimes gerenciados e material da CA podem permanecer separados dos binários. Veja os diretórios reais em [Configuração](/docs/configuration), incluindo eventual `LOCDEV_HOME`. Faça backup e examine cada diretório antes de qualquer exclusão manual. Não apague projetos, instalações externas de PHP/Node ou diretórios compartilhados.

Reabra o navegador após remover a confiança TLS e confira as preferências de inicialização do sistema. Para reinstalar, siga [Instalação](/docs/installation); não presuma que dados de uma versão diferente serão compatíveis.
