## Documento de Requisitos do Produto (PRD) - lcode

### 1. Visão Geral do Produto

O **lcode** é um editor de código-fonte desktop projetado para desenvolvedores que buscam uma ferramenta **super leve, minimalista e altamente personalizável**. Seu principal objetivo é oferecer uma experiência de edição de código eficiente e sem distrações, com foco especial na capacidade de abrir e manipular **grandes arquivos de código e JSON sem travamentos ou sobrecarga do sistema**, uma dor comum em editores robustos como PHPStorm. O lcode será a ferramenta ideal para o desenvolvedor que valoriza performance e um ambiente de trabalho adaptado às suas necessidades.

---

### 2. Público-alvo

O lcode é primariamente desenvolvido para **você, o principal usuário**. Ele é ideal para desenvolvedores de software que utilizam sistemas operacionais baseados em Linux (como Pop!_OS) e que:
* Buscam um editor de código com **alto desempenho**, especialmente ao lidar com arquivos de grande porte.
* Valorizam um ambiente de desenvolvimento **minimalista e sem "inchaço"**, com apenas os recursos essenciais.
* Desejam **personalizar** o editor de forma profunda para se adequar ao seu fluxo de trabalho e preferências visuais.
* Estão cansados de editores que consomem muitos recursos e travam.

---

### 3. Recursos Principais (Features)

Aqui vamos listar as funcionalidades essenciais que o lcode precisa ter. Estes são os "o que" do editor, sem detalhar "como" (isso vem depois na documentação técnica).

* **Gerenciamento de Arquivos e Diretórios:**
    * Capacidade de **abrir um diretório** (pasta) específico, exibindo sua estrutura de pastas e arquivos.
    * Exibição clara e intuitiva da **árvore de diretórios** (sidebar de navegação).
    * Permitir **clicar em um arquivo** na árvore para abri-lo no painel de edição.
    * Navegação e abertura rápida de arquivos.
* **Edição de Código:**
    * **Visualização do conteúdo** do arquivo de texto ou código selecionado.
    * Permitir **edição livre** do conteúdo do arquivo.
    * **Salvar alterações** nos arquivos.
    * Suporte básico para **realce de sintaxe (syntax highlighting)** para linguagens de programação comuns (ex: JavaScript, Python, JSON, Markdown). *Isso pode ser simples no início, sem a complexidade de um VS Code.*
* **Terminal Integrado:**
    * Um **terminal de linha de comando** que pode ser aberto e usado dentro do próprio editor.
    * Funcionalidade básica de um terminal (executar comandos, exibir saída).
    * Capacidade de abrir o terminal no diretório do projeto atualmente aberto.

---

### 4. Requisitos de Performance

Este é um ponto crucial para o lcode, diferenciando-o dos outros editores.

* **Abertura e Manipulação de Arquivos Grandes:**
    * O editor deve ser capaz de **abrir e editar arquivos de texto/código/JSON com centenas de megabytes (ou até gigabytes)** sem travar, congelar ou consumir excessivamente a RAM/CPU do computador.
    * A rolagem (scroll) e a navegação nesses arquivos devem ser **fluidas e responsivas**.
    * A função de salvar arquivos grandes deve ser **rápida**.
* **Baixo Consumo de Recursos:**
    * O lcode deve ter um **baixo consumo de memória RAM e CPU** em estado ocioso e durante o uso normal (edição de arquivos menores).
    * A inicialização do editor deve ser **extremamente rápida**.

---

Este é um bom começo para o seu PRD! Cobrimos o que o lcode é, para quem ele é e o que ele precisa fazer, com um foco especial na performance.
