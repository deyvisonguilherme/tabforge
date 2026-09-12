# TabForge 🎸🎼

**TabForge** é um motor musical e transcritor de áudio em **Rust**, projetado com **Clean Architecture** e **Domain-Driven Design (DDD)** para transformar áudios (MP3, WAV, FLAC) e arquivos MIDI em uma representação musical intermediária (**Music IR**) e gerar partituras (MusicXML), tablaturas para guitarra/baixo (ASCII) e arquivos MIDI com otimização ergonômica de digitação.

---

## 🏛️ Arquitetura do Workspace

O projeto é organizado como um **Cargo Workspace** modular:

```text
tabforge/
├── Cargo.toml                  # Workspace raiz
├── tabforge.toml               # Configuração padrão de áudio e quantização
├── README.md                   # Documentação arquitetural
│
├── crates/
│   ├── tabforge-core/          # Music IR (Song, Track, Measure, Beat, Note, Pitch, Tempo)
│   │                           # - Independente de áudio, MIDI e codecs.
│   │                           # - Aritmética temporal racional exata (num-rational).
│   │
│   ├── tabforge-guitar/        # Motor de guitarra e baixo
│   │                           # - Fretboard mapper e afinações (Standard, Drop D, Bass).
│   │                           # - Otimizador de digitação por Programação Dinâmica / Viterbi.
│   │                           # - Dicionário de acordes.
│   │
│   ├── tabforge-audio/         # DSP e decodificação de áudio
│   │                           # - Symphonia (decodificador universal puro Rust).
│   │                           # - Pitch Detection (YIN/Autocorrelação).
│   │                           # - Onset Detection e Beat Grid / Quantização.
│   │                           # - Source Separator trait (pronto para ONNX/Demucs).
│   │
│   ├── tabforge-midi/          # Leitura e escrita MIDI
│   │                           # - Midly (parser/writer de alta performance).
│   │                           # - Conversão bidirecional MIDI <-> Music IR.
│   │
│   ├── tabforge-score/         # Exportadores de notação e tablatura
│   │                           # - MusicXML 4.0 (compatível com MuseScore, Guitar Pro).
│   │                           # - Tablatura ASCII com técnicas de execução.
│   │
│   └── tabforge-cli/           # CLI unificada (clap + tracing + toml)
│                               # - Comandos: new, inspect, transcribe, midi, score, tab.
```

---

## 🚀 Fluxo de Processamento (Pipeline)

```text
   ┌───────────────┐
   │ MP3/WAV/FLAC  │
   └───────┬───────┘
           │ (Symphonia)
           ▼
   ┌───────────────┐
   │ AudioBuffer   │
   └───────┬───────┘
           │ (YIN Pitch + Onsets + Beat Grid)
           ▼
   ┌───────────────┐
   │  Quantizer    │
   └───────┬───────┘
           │
           ▼
   ┌───────────────────────┐
   │       Music IR        │
   │ (Song, Measure, Beat) │
   └───────┬───────────────┘
           │
   ┌───────┼────────────────────────┐
   ▼       ▼                        ▼
┌──────┐┌───────────────┐     ┌───────────────┐
│ MIDI ││   MusicXML    │     │ Fingering DP  │
└──────┘│ (MuseScore/GP)│     └───────┬───────┘
        └───────────────┘             │
                                      ▼
                              ┌───────────────┐
                              │   ASCII TAB   │
                              └───────────────┘
```

---

## 🛠️ Comandos da CLI

```bash
# 1. Iniciar novo projeto
cargo run -p tabforge-cli -- new minha-musica

# 2. Inspecionar arquivo de áudio (taxa de amostragem, canais, duração)
cargo run -p tabforge-cli -- inspect audio.wav

# 3. Transcrever áudio diretamente para tablatura
cargo run -p tabforge-cli -- transcribe audio.wav --instrument guitar

# 4. Gerar arquivo MIDI a partir do áudio / Music IR
cargo run -p tabforge-cli -- midi audio.wav -o output.mid

# 5. Exportar partitura em MusicXML 4.0
cargo run -p tabforge-cli -- score audio.wav -o score.musicxml

# 6. Renderizar escala e tablatura ASCII no terminal
cargo run -p tabforge-cli -- tab audio.wav
```

---

## 🧪 Testes

```bash
cargo test --workspace
```
