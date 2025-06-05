lcode/
├── Cargo.toml
├── src/
│   ├── main.rs               # Ponto de entrada da aplicação
│   ├── lib.rs                # Declaração dos módulos da crate
│   ├── ui/                   # Módulo para a camada de interface do usuário (egui)
│   │   ├── mod.rs
│   │   └── app.rs            # Lógica principal da aplicação egui
│   │   └── widgets.rs        # Componentes UI reutilizáveis (se necessário)
│   ├── core/                 # Módulo principal do editor
│   │   ├── mod.rs
│   │   ├── editor.rs         # Lógica de edição, cursor, desfazer/refazer
│   │   └── buffer.rs         # Gerenciamento do buffer de texto (ex: Ropey)
│   │   └── file_handler.rs   # Abertura, leitura e salvamento de arquivos
│   ├── terminal/             # Módulo para o terminal integrado
│   │   ├── mod.rs
│   │   └── pty_integration.rs # Integração com pseudoterminais
│   │   └── shell_handler.rs  # Manipulação do shell
│   ├── file_explorer/        # Módulo para o explorador de arquivos
│   │   ├── mod.rs
│   │   └── fs_tree.rs        # Estrutura de dados da árvore de arquivos
│   │   └── scanner.rs        # Lógica de escaneamento de diretórios
│   ├── syntax_highlighting/  # Módulo para o realce de sintaxe
│   │   ├── mod.rs
│   │   └── highlighter.rs    # Lógica de realce
│   │   └── themes.rs         # Definição de temas de cores
│   ├── utils/                # Utilitários e helper functions gerais
│   │   └── mod.rs
│   │   └── performance.rs    # Funções relacionadas a otimizações de performance
│   │   └── constants.rs      # Constantes globais
│   └── config/               # Módulo para gerenciamento de configurações (temas, atalhos)
│       └── mod.rs
│       └── settings.rs
└── tests/
    └── integration_tests.rs
    └── unit_tests/
        └── core_tests.rs