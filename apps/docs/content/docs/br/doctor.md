# Solução de problemas

Comece sempre por:

```bash
locdev doctor
```

![Doctor](screenshot:doctor)

## O que é verificado

| Verificação                   | O que significa                                                                                                                                                            |
| ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Daemon                        | Se a CLI consegue falar com o `locdevd`, e se as versões do protocolo batem.                                                                                               |
| Configuration folder          | Se a pasta de configuração pode ser escrita.                                                                                                                               |
| Project folders               | Se as pastas registradas ainda existem.                                                                                                                                    |
| HTTP proxy / HTTPS proxy      | Se o proxy está escutando e se precisou cair para a porta de fallback (e por quê).                                                                                         |
| HTTP port 80 / HTTPS port 443 | Se as portas estão livres, em uso por outro programa ou pelo próprio LocDev.                                                                                               |
| Local CA                      | Se o sistema confia na CA local.                                                                                                                                           |
| Browser certificate stores    | Se Chrome/Firefox (Linux) confiam na CA.                                                                                                                                   |
| DNS server                    | Se o servidor `*.test` está no ar.                                                                                                                                         |
| .test resolver                | Para onde `*.test` realmente vai: LocDev, outra ferramenta ou lugar nenhum.                                                                                                |
| PHP, Composer, Node.js, npm   | Se as ferramentas estão no `PATH` (e se algum site registrado precisa delas).                                                                                              |
| Runtimes de cada site         | Versão fixada que não está instalada, versão fora da faixa que o projeto declara, versão sem suporte do publicador, gerenciador de pacotes que exige um runtime mais novo. |
| Default de cada runtime       | Se o default definido em `runtimes.toml` ainda está instalado.                                                                                                             |
| Secrets vault                 | Se a chave do cofre de variáveis secretas está acessível (keyring ou arquivo).                                                                                             |
| Framework definitions         | A revisão das definições em uso, a origem (embutida ou baixada com assinatura verificada) e a última verificação.                                                          |

## Problemas comuns

### `the daemon is not running`

```bash
locdev start
```

Se não subir, veja o log do daemon, `logs/locdevd.log` na pasta de dados (no Linux, `~/.local/share/locdev/logs/locdevd.log`; a tabela de pastas está em [Configuração](/docs/configuration)).

### Porta 80/443 em uso

Outra ferramenta (Apache, nginx, Valet, Herd, Yerd…) está ocupando a porta. O LocDev passa a usar a 7080/7443 e mostra isso na barra de status. Libere a porta ou mude `proxy.http_port` / `tls.https_port`.

### `*.test → another tool`

Outra ferramenta configurou o resolvedor para `.test`. O `doctor` mostra o arquivo dela. Remova-o se não usa mais a ferramenta e rode `locdev dns install`.

### O navegador ainda avisa sobre o certificado

1. Confira se `locdev tls status` mostra `system trusted` e `browsers trusted`.
2. Reinicie o navegador por completo.
3. No Linux, instale `libnss3-tools` e rode `locdev tls trust` de novo.

### Um projeto Vite responde "Blocked request"

Nos processos que o LocDev inicia, os nomes do site já são liberados (veja [Servidores de desenvolvimento](/docs/servers)). Se você roda o Vite por fora do LocDev, adicione `server: { allowedHosts: [".test"] }` ao `vite.config`, ou use a URL `.localhost`.

### O site falha ao iniciar

Abra o site no app ou rode `locdev logs <nome>`. A saída do processo mostra o erro, por exemplo uma extensão do PHP faltando ou uma dependência não instalada.
