## Documento de Requisitos Técnicos (TRD) - lcode

### 1. Escolha de Tecnologias

* **Linguagem de Programação Principal:** Rust
    * **Justificativa:** Selecionada por sua performance de ponta, segurança de memória sem garbage collector (essencial para baixo consumo de recursos e manipulação de arquivos grandes), forte suporte a concorrência e um ecossistema crescente de bibliotecas.
* **Framework de Interface Gráfica (UI):** egui (com potencial para Slint/iced no futuro, se necessário)
    * **Justificativa:** egui oferece uma abordagem "immediate mode GUI" que resulta em uma interface super leve e responsiva, com baixo consumo de recursos. É ideal para uma aplicação minimalista e permite controle preciso sobre a renderização, crucial para um editor de texto de alta performance.
* **Sistema Operacional Alvo:** Linux (com foco inicial em Pop!_OS e compatibilidade com outras distribuições baseadas em glibc).

---

### 2. Arquitetura Geral (Visão de Alto Nível)

O lcode terá uma arquitetura modular para garantir flexibilidade, manutenção e, principalmente, performance.

* **Core do Editor (Rust):** O coração do editor, responsável por:
    * Manipulação de arquivos (abrir, ler, salvar).
    * Gerenciamento de buffer de texto (como o texto do arquivo é armazenado e manipulado em memória).
    * Lógica de busca e substituição (futuro).
    * Integração com o terminal.
* **Camada de UI (Rust + egui):** Responsável por:
    * Renderizar a interface gráfica: barra lateral de arquivos, painel de edição, terminal.
    * Capturar eventos do usuário (cliques do mouse, digitação do teclado).
    * Comunicar-se com o Core do Editor para exibir e manipular dados.
* **Módulo de Terminal (Rust):** Uma sub-componente que emula um terminal, comunicando-se com o shell do sistema operacional.

```
+---------------------+      +---------------------+
|                     |      |                     |
|     CAMADA DE UI    |<---->|    CORE DO EDITOR   |
|    (Rust + egui)    |      |       (Rust)        |
|                     |      |                     |
+----------^----------+      +----------^----------+
           |                              |
           |                              |
+----------+----------+      +----------+----------+
|                     |      |                     |
|  Gerenciamento de   |<---->|  Manipulação de     |
|     Eventos UI      |      |     Buffer de Texto |
|                     |      |                     |
+---------------------+      +---------------------+
           |
           |
+----------v----------+
|                     |
|  MÓDULO DE TERMINAL |
|       (Rust)        |
|                     |
+---------------------+
```

---

### 3. Estratégias de Performance para Arquivos Grandes

Este é o ponto mais crítico do TRD e o grande diferencial do lcode.

* **Leitura de Arquivos sob Demanda (Lazy Loading / Memory Mapping):**
    * Ao abrir um arquivo grande, **não carregar o arquivo inteiro na memória RAM de uma vez**.
    * **Estratégia 1 (Memory Mapping - `mmap`):** Utilizar mapeamento de memória (se suportado pelo sistema operacional, como no Linux). Isso permite que o sistema operacional gerencie o carregamento de partes do arquivo conforme elas são acessadas, como se estivesse na memória, mas sem carregar tudo.
    * **Estratégia 2 (Leitura em Chunks/Blocos):** Ler apenas as partes do arquivo que são visíveis na tela (e um pouco mais acima e abaixo para rolagem suave). Quando o usuário rola, novas partes são lidas do disco.
    * **Gerenciamento de Buffer:** O buffer de texto do editor não deve ser um simples `String` gigante. Deve ser uma estrutura de dados otimizada para lidar com grandes volumes de texto, permitindo inserções, exclusões e acessos rápidos a linhas específicas sem copiar grandes blocos de memória (ex: Rope, Gap Buffer). Rust tem crates que implementam isso.
* **Renderização Otimizada da UI (egui):**
    * **Desenho Incremental:** O egui, sendo *immediate mode*, já favorece isso. Apenas as partes da tela que mudaram são redesenhadas.
    * **Virtualização de Linhas:** Ao exibir um arquivo grande, **não renderizar todas as linhas**. Renderizar apenas as linhas que estão visíveis na viewport (janela de rolagem) e um pequeno buffer acima e abaixo.
    * **Otimização de Realce de Sintaxe:** O realce de sintaxe deve ser feito apenas para as linhas visíveis e ser o mais performático possível, evitando reprocessar todo o arquivo a cada digitação.
* **Gerenciamento de Memória (Rust):**
    * A natureza do Rust de não ter coletor de lixo e permitir controle manual sobre a memória significa que podemos evitar alocações desnecessárias e reduzir a pegada de memória.
    * Evitar cópias de dados sempre que possível, usando referências e borrows do Rust.
* **Processamento Assíncrono/Paralelo:**
    * Tarefas que podem ser demoradas (como indexação inicial de um diretório muito grande, busca global, ou até mesmo algumas partes do realce de sintaxe) podem ser executadas em threads separadas para não bloquear a interface do usuário. Rust tem ótimas ferramentas para concorrência segura (`async/await`, `rayon`).

---

### 4. Componentes e Módulos Principais (Detalhes Iniciais)

* **File System Explorer (Explorador de Arquivos):**
    * Responsável por escanear o diretório selecionado.
    * Manter uma representação em memória da estrutura de pastas/arquivos (otimizada para grandes diretórios, talvez lendo o conteúdo das pastas apenas quando expandidas).
    * Interagir com a UI para exibir a árvore e lidar com cliques.
* **Text Editor Core (Núcleo do Editor de Texto):**
    * **Buffer de Texto:** Implementar (ou usar uma crate) uma estrutura de dados eficiente para o texto (ex: `ropey` crate em Rust, que é um *rope*).
    * **Lógica de Edição:** Inserção, deleção, seleção de texto.
    * **Cursor:** Gerenciamento da posição do cursor.
    * **Desfazer/Refazer (Undo/Redo):** Sistema eficiente de histórico de alterações.
* **Syntax Highlighter (Realce de Sintaxe):**
    * Recebe o texto de uma linha e aplica regras para identificar palavras-chave, strings, comentários, etc.
    * Pode usar uma biblioteca externa (ex: `tree-sitter` bindings para Rust, ou algo mais simples como `syntect`). Priorizar performance.
* **Integrated Terminal (Terminal Integrado):**
    * Spawna um processo de shell (ex: `bash`, `zsh`) e se comunica com ele (envia comandos, recebe saída).
    * Renderiza o texto do terminal na UI.
    * Gerencia o histórico de comandos e o input do usuário.
    * Pode usar crates como `pty` ou `tokio-pty` para lidar com pseudoterminais.

---

Este é um esqueleto robusto para o seu TRD. Ele define as tecnologias, a arquitetura e as estratégias para atingir seus objetivos de performance e minimalismo.
