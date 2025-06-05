## Documento de Requisitos Funcionais (FRD) - lcode

### 1. Requisitos de Gerenciamento de Arquivos e Diretórios

**FR.1.1: Abertura de Diretórios**
* **Descrição:** O lcode deve permitir que o usuário selecione e abra um diretório (pasta) local para navegar em seu conteúdo.
* **Comentários:** Pode ser feito através de um diálogo de seleção de pasta (file picker) ou arrastando e soltando uma pasta no editor.

**FR.1.2: Exibição da Árvore de Diretórios (Sidebar)**
* **Descrição:** O lcode deve exibir uma barra lateral (sidebar) com a estrutura hierárquica (em árvore) do diretório aberto.
* **Detalhes:**
    * **FR.1.2.1:** Pastas devem ser clicáveis para expandir/recolher seu conteúdo.
    * **FR.1.2.2:** Arquivos devem ser exibidos dentro de suas pastas.
    * **FR.1.2.3:** Pastas e arquivos devem ter ícones visuais distintos para fácil identificação.

**FR.1.3: Abertura de Arquivos para Edição**
* **Descrição:** O lcode deve permitir que o usuário clique em um arquivo na árvore de diretórios para abrir seu conteúdo no painel de edição principal.
* **Detalhes:**
    * **FR.1.3.1:** Cada arquivo aberto deve ser exibido em uma nova "aba" ou "buffer" (como a maioria dos editores faz) para permitir alternar entre eles.
    * **FR.1.3.2:** Se o arquivo já estiver aberto, ao clicar, ele deve apenas focar na aba/buffer existente.

**FR.1.4: Criação de Novos Arquivos/Pastas (futuro, se houver tempo)**
* **Descrição:** O lcode deve permitir que o usuário crie novos arquivos e pastas dentro do diretório aberto.
* **Comentários:** Inicialmente, pode ser um recurso mais avançado e implementado em uma fase posterior, para manter o foco no minimalismo.

**FR.1.5: Renomear/Excluir Arquivos/Pastas (futuro, se houver tempo)**
* **Descrição:** O lcode deve permitir que o usuário renomeie e exclua arquivos e pastas.
* **Comentários:** Similar ao FR.1.4, pode ser um recurso para depois.

---

### 2. Requisitos de Edição de Código

**FR.2.1: Visualização do Conteúdo do Arquivo**
* **Descrição:** O painel de edição principal deve exibir o conteúdo completo do arquivo aberto.
* **Detalhes:**
    * **FR.2.1.1:** O conteúdo deve ser exibido como texto puro, com quebras de linha corretas.
    * **FR.2.1.2:** Deve haver uma barra de rolagem vertical (scroll bar) para navegar por arquivos maiores que a tela.

**FR.2.2: Edição de Texto**
* **Descrição:** O usuário deve ser capaz de digitar, apagar e modificar o texto no painel de edição.
* **Detalhes:**
    * **FR.2.2.1:** Suporte para cursor de texto que se move com as setas do teclado.
    * **FR.2.2.2:** Suporte para seleção de texto com o mouse e teclado.
    * **FR.2.2.3:** Suporte para as operações básicas de Ctrl+C (copiar), Ctrl+V (colar), Ctrl+X (recortar).
    * **FR.2.2.4:** Suporte para Ctrl+Z (desfazer) e Ctrl+Y (refazer) para as últimas operações de edição.

**FR.2.3: Salvamento de Arquivos**
* **Descrição:** O lcode deve permitir que o usuário salve as alterações feitas em um arquivo.
* **Detalhes:**
    * **FR.2.3.1:** Uma indicação visual (ex: um asterisco no nome da aba) deve ser exibida quando um arquivo tiver alterações não salvas.
    * **FR.2.3.2:** O usuário deve ser capaz de salvar o arquivo usando um atalho de teclado (ex: Ctrl+S).
    * **FR.2.3.3:** Ao tentar fechar um arquivo com alterações não salvas, o editor deve perguntar ao usuário se deseja salvar, descartar ou cancelar.

**FR.2.4: Realce de Sintaxe (Syntax Highlighting)**
* **Descrição:** O lcode deve aplicar realce de sintaxe básico ao código aberto, baseado na extensão do arquivo.
* **Detalhes:**
    * **FR.2.4.1:** Suporte inicial para realce de sintaxe para arquivos `.json`, `.js`, `.py`, `.md` e `.txt` (como texto puro).
    * **FR.2.4.2:** Diferentes tipos de elementos (palavras-chave, strings, comentários, números) devem ser exibidos com cores distintas.

**FR.2.5: Números de Linha**
* **Descrição:** O editor deve exibir números de linha à esquerda do painel de edição para cada linha do arquivo.

---

### 3. Requisitos do Terminal Integrado

**FR.3.1: Abertura e Fechamento do Terminal**
* **Descrição:** O lcode deve permitir que o usuário abra e feche um painel com um terminal integrado.
* **Detalhes:**
    * **FR.3.1.1:** O terminal deve ser acessível através de um atalho de teclado ou um botão na interface.
    * **FR.3.1.2:** O painel do terminal deve ser redimensionável verticalmente (ex: arrastando a borda superior).

**FR.3.2: Execução de Comandos**
* **Descrição:** O terminal integrado deve permitir que o usuário digite e execute comandos da linha de comando, como em um terminal externo.
* **Detalhes:**
    * **FR.3.2.1:** A saída dos comandos deve ser exibida no painel do terminal.
    * **FR.3.2.2:** O terminal deve iniciar no diretório raiz do projeto aberto no lcode.

**FR.3.3: Interação Básica do Terminal**
* **Descrição:** O terminal deve suportar operações básicas como rolar a saída, copiar e colar texto da saída.

---

### 4. Requisitos de Performance (Funcionais)

Estes são os requisitos do PRD traduzidos para o que o usuário *experimentará* em termos funcionais.

**FR.4.1: Abertura Rápida de Arquivos Grandes**
* **Descrição:** O lcode deve abrir arquivos de texto com múltiplos gigabytes (GB) de tamanho em questão de segundos.
* **Critério de Aceitação:** Para um arquivo de 1GB de texto puro, a abertura não deve exceder 5 segundos em um sistema com especificações razoáveis.

**FR.4.2: Rolagem Fluida em Arquivos Grandes**
* **Descrição:** A rolagem vertical em arquivos de múltiplos gigabytes deve ser suave e responsiva, sem atrasos perceptíveis ou congelamentos da interface.
* **Critério de Aceitação:** Nenhuma queda visível de FPS (Frames Per Second) ou travamento ao rolar rapidamente em arquivos de 1GB.

**FR.4.3: Baixo Consumo de Recursos em Ocioso**
* **Descrição:** Quando o lcode estiver aberto, mas sem edição ativa, ele deve consumir uma quantidade mínima de memória RAM e CPU.
* **Critério de Aceitação:** Menos de 100MB de RAM consumidos em ocioso (sem arquivos grandes abertos) e uso de CPU próximo de 0%.

---

### 5. Requisitos de Personalização (Funcionais Iniciais)

**FR.5.1: Temas Visuais**
* **Descrição:** O usuário deve ser capaz de selecionar diferentes temas visuais para o editor (ex: tema claro, tema escuro).
* **Detalhes:**
    * **FR.5.1.1:** As cores de fundo, texto, realce de sintaxe e da interface geral devem mudar de acordo com o tema.

**FR.5.2: Configuração de Atalhos de Teclado (futuro, se houver tempo)**
* **Descrição:** O usuário deve ser capaz de remapear atalhos de teclado para as funcionalidades do editor.

---

Este FRD detalha as funcionalidades que o lcode precisa ter. Cada item aqui é uma funcionalidade que o usuário poderá ver e interagir.