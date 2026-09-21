# `help` — Flows

## User flow

```
flowchart LR
  U[Developer] -->|archspec help [topic]| H[help command]
  H -->|no topic| IDX[print topic index to stdout]
  IDX --> U
  H -->|topics| IDX
  H -->|command name| CMD[print that command's --help to stdout]
  H -->|glob / spec / constraints / languages / workflow / diagnostics| TOPIC[print the topic block to stdout]
  H -->|unknown topic| ERR[error naming the topic, valid topics, and a hint]
  CMD --> U
  TOPIC --> U
  ERR --> U
```

The developer asks the tool for any piece of its model-driven manual: the topic
index, a topic, or a command's own help. Everything lands on stdout for piping
or `>`-redirect.

## Application flow

```
flowchart TD
  START[dispatch: help + args] --> GHELP{args contain --help?}
  GHELP -- yes --> INDEX[print commands::help::HELP - the topic index]
  GHELP -- no --> PARSE[parse args; zero or one positional]
  PARSE --> UNKNOWN{unknown flag?}
  UNKNOWN -- yes --> ERR1[error naming the flag, non-zero exit]
  PARSE --> POS{two positionals?}
  POS -- yes --> ERR2[error: at most one path argument]
  POS -- no --> TOPIC{topic?}
  TOPIC -- none --> INDEX
  TOPIC -- topics --> INDEX
  TOPIC -- commands --> REGISTRY[render one block per registry command]
  TOPIC -- spec --> SPEC[print commands::spec::HELP]
  TOPIC -- glob/constraints/workflow/diagnostics --> CONST[print the embedded topic constant]
  TOPIC -- languages --> TIERS[render LANGUAGE_TIERS]
  TOPIC -- other --> CMD{command in registry?}
  CMD -- yes --> CMDOUT[print that command's --help]
  CMD -- no --> ERR3[error naming the topic, valid topics list, hint]
  SPEC --> EXIT0[exit 0]
  REGISTRY --> EXIT0
  CONST --> EXIT0
  TIERS --> EXIT0
  INDEX --> EXIT0
  CMDOUT --> EXIT0
```

The command parses its arguments, then prints the selected constant or a
registry-rendered block. No project, spec file, or toolchain is touched; the
output is fully deterministic.