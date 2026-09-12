# TabForge 🎸🎼

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-passing-brightgreen.svg)]()

**TabForge** é um motor musical de transcrição de áudio e sintetizador de tablaturas em **Rust**, projetado para converter gravações de instrumentos (MP3, WAV, FLAC, OGG, AAC) e arquivos MIDI em uma representação musical estruturada (**Music IR**). O sistema gera tablaturas ergonômicas para guitarra/baixo (ASCII), partituras completas (MusicXML 4.0) e arquivos MIDI, integrando separação de faixas (stems) e detecção de técnicas de execução.

---

## 📑 Sumário

- [Principais Funcionalidades](#-principais-funcionalidades)
- [Instalação & Compilação](#-instalação--compilação)
- [Quickstart (Guia Rápido)](#-quickstart-guia-rápido)
- [Referência de Comandos da CLI](#-referência-de-comandos-da-cli)
  - [Opções Globais](#opções-globais)
  - [`transcribe`](#1-transcribe---transcrição-de-áudio-para-tablaturapartitura)
  - [`stems`](#2-stems---separação-de-fontes-e-instrumentos)
  - [`models`](#3-models---gerenciador-de-modelos-neurais)
  - [`inspect`](#4-inspect---diagnóstico-e-metadados-de-áudio)
  - [`tab`](#5-tab---renderização-de-tablatura-ascii)
  - [`score`](#6-score---exportação-de-partitura-musicxml)
  - [`midi`](#7-midi---conversão-e-geração-midi)
  - [`new`](#8-new---inicialização-de-novo-projeto)
- [Notação de Técnicas na Tablatura](#-notação-de-técnicas-na-tablatura)
- [Configuração (`tabforge.toml`)](#-configuração-tabforgetoml)
- [Arquitetura do Workspace](#-arquitetura-do-workspace)
- [Testes Automatizados](#-testes-automatizados)

---

## ✨ Principais Funcionalidades

- **Decodificação de Áudio Universal:** Suporte nativo e puro-Rust a múltiplos formatos (MP3, WAV, FLAC, OGG, AAC) com resoluções de 8, 16, 24 e 32-bit (inteiro e ponto flutuante), downmixing inteligente ITU-R BS.775 para estéreo/surround 5.1/7.1 e reamostragem cúbica Catmull-Rom para 44.1 kHz.
- **Separação de Fontes (Stems):** Algoritmo HPSS (Harmonic-Percussive Source Separation) por filtragem de mediana 2D e iSTFT overlap-add, integrado a suporte para modelos neurais profundos (HTDemucs v4 6-stems / BS-Roformer) em janelas deslizantes com crossfades suaves.
- **DSP e Pitch Tracking de Alta Performance:** Algoritmo YIN acelerado por FFT ($O(W \log W)$) com interpolação parabólica para detecção contínua e precisa de fundamental ($f_0$).
- **Detecção de Articulações e Expressão de Guitarra:** Reconhecimento automático de Bends, Release Bends, Vibrato ($4-8.5\text{ Hz}$), Slides, Hammer-ons, Pull-offs e Palm Mutes a partir de envelopes e trajetórias de pitch.
- **Segmentação ADSR e Quantização Métrica:** Segmentador de dinâmicas de ataque, sustain e release conectado a quantizador rítmico que preenche pausas automaticamente em compassos estritos.
- **Otimizador Biomecânico de Digitação (Viterbi):** Algoritmo de Programação Dinâmica que mapeia notas para as melhores posições no braço do instrumento (fretboard), minimizando o esforço físico da mão e saltos de trastes.
- **Exportação Multi-formato:** Tablatura ASCII de alta legibilidade, partituras no padrão MusicXML 4.0 (compatíveis com Guitar Pro, MuseScore, Finale, Sibelius) e arquivos Standard MIDI.

---

## 📦 Instalação & Compilação

Certifique-se de ter o [Rust Toolchain (Rust 1.75+)](https://rustup.rs/) instalado.

```bash
# Clonar o repositório
git clone https://github.com/deyvisonguilherme/tabforge.git
cd tabforge

# Compilar o binário em modo release
cargo build --release

# O executável estará disponível em ./target/release/tabforge
./target/release/tabforge --help
```

---

## ⚡ Quickstart (Guia Rápido)

```bash
# 1. Transcrever um solo de guitarra com separação de áudio para tablatura
cargo run -p tabforge-cli -- transcribe solo.mp3 --separate --stem guitar

# 2. Separar uma música em faixas individuais (guitar, bass, drums, vocals, other)
cargo run -p tabforge-cli -- stems musica.mp3 -o ./meus_stems

# 3. Exportar partitura para abrir no Guitar Pro ou MuseScore
cargo run -p tabforge-cli -- score musica.mp3 -o partitura.musicxml

# 4. Transcrever música com fórmula de compasso personalizada (ex: 3/4, 6/8, 7/8)
cargo run -p tabforge-cli -- transcribe riff.wav --time-signature 7/8
```

---

## 📖 Referência de Comandos da CLI

### Opções Globais

| Flag | Descrição |
| :--- | :--- |
| `-c, --config <FILE>` | Caminho para um arquivo de configuração TOML personalizado. |
| `-v, --verbose` | Aumenta o nível de verbosidade dos logs (`-v` para Info/Debug, `-vv` para Trace). |
| `-h, --help` | Exibe a ajuda da linha de comando. |
| `-V, --version` | Exibe a versão atual do TabForge. |

---

### 1. `transcribe` - Transcrição de Áudio para Tablatura/Partitura

Conecta todo o pipeline de transcrição: decodificação, separação de stem (opcional), detecção de onsets, tracking de pitch, segmentação ADSR, quantização rítmica e otimização de digitação.

```bash
tabforge transcribe <INPUT> [OPTIONS]
```

#### Parâmetros e Opções:

| Argumento / Flag | Padrão | Descrição |
| :--- | :--- | :--- |
| `<INPUT>` | *(Obrigatório)* | Caminho para o arquivo de áudio de entrada (`.mp3`, `.wav`, `.flac`, etc.). |
| `-i, --instrument <NAME>` | `guitar` | Instrumento alvo para otimização de digitação (`guitar`, `bass`). |
| `-o, --output <FILE>` | *None* | Arquivo de saída opcional. O formato é inferido pela extensão (`.musicxml`, `.mid`, `.txt`). Se omitido, imprime a tablatura ASCII no terminal. |
| `--separate` | `false` | Ativa a separação de fontes (isolamento do instrumento) antes de transcrever. |
| `--stem <STEM>` | `guitar` | Stem a ser transcrito se `--separate` estiver ativo (`guitar`, `bass`, `vocals`, `other`). |
| `--time-signature <TS>` | *Auto* | Força uma fórmula de compasso manual (ex: `4/4`, `3/4`, `6/8`, `7/8`), ajustando a quantização e os limites de compasso. |

#### Exemplos de Uso:

```bash
# Transcrição direta imprimindo tablatura no terminal:
tabforge transcribe solo.wav

# Transcrever isolando a guitarra de uma mix completa:
tabforge transcribe musica.mp3 --separate --stem guitar

# Transcrever linha de baixo e salvar em arquivo MIDI:
tabforge transcribe musica.flac --separate --stem bass -i bass -o baixo.mid

# Transcrever e gerar arquivo MusicXML para Guitar Pro:
tabforge transcribe solo.mp3 --separate -o solo.musicxml

# Transcrever uma valsa em 3/4 com quantização métrica adaptada:
tabforge transcribe valsa.mp3 --time-signature 3/4

# Transcrever riff progressivo em 7/8 com separação de stem e exportação para MusicXML:
tabforge transcribe prog_riff.wav --time-signature 7/8 --separate --stem guitar -o prog.musicxml
```

---

### 2. `stems` - Separação de Fontes e Instrumentos

Separa o arquivo de áudio em stems individuais (Guitarras, Baixo, Bateria, Vocais e Outros instrumentos) e exporta em arquivos WAV de alta fidelidade (PCM 16-bit a 44.1 kHz).

```bash
tabforge stems <INPUT> [OPTIONS]
```

#### Parâmetros e Opções:

| Argumento / Flag | Padrão | Descrição |
| :--- | :--- | :--- |
| `<INPUT>` | *(Obrigatório)* | Arquivo de áudio a ser separado. |
| `-o, --output-dir <DIR>` | `./stems` | Diretório de destino onde os arquivos WAV separados serão salvos. |
| `--model <PATH>` | *None* | Caminho para um arquivo de pesos de modelo ONNX neural personalizado. |

#### Exemplos de Uso:

```bash
# Separar stems na pasta padrão (./stems):
tabforge stems musica.mp3

# Separar em pasta personalizada:
tabforge stems trilha.wav -o ./meus_stems

# Utilizar um modelo ONNX customizado:
tabforge stems mix.flac -o ./stems --model ~/.cache/tabforge/models/htdemucs_6s.onnx
```

---

### 3. `models` - Gerenciador de Modelos Neurais

Gerencia o catálogo, download e cache de modelos pré-treinados de separação neural (HTDemucs v4 e BS-Roformer).

```bash
tabforge models <SUBCOMMAND>
```

#### Subcomandos:

- `list`: Lista todos os modelos conhecidos e verifica se já estão baixados no cache local.
- `download [MODEL_NAME]`: Baixa os pesos de um modelo para o diretório de cache (`~/.cache/tabforge/models`).
- `path`: Exibe o caminho absoluto do diretório de cache de modelos.

#### Exemplos de Uso:

```bash
# Ver modelos disponíveis e status de instalação:
tabforge models list

# Baixar o modelo HTDemucs v4 6-stems:
tabforge models download htdemucs_6s

# Baixar o modelo BS-Roformer:
tabforge models download bs_roformer

# Verificar localização da pasta de modelos:
tabforge models path
```

---

### 4. `inspect` - Diagnóstico e Metadados de Áudio

Analisa detalhadamente as propriedades do arquivo de áudio, metadados de codec e energia de sinal.

```bash
tabforge inspect <INPUT>
```

#### Exemplo de Saída:
```text
=== Audio Inspection ===
File:       audio.mp3
Duration:   3m 42s (222.45s)
Channels:   2 (Stereo)
SampleRate: 44100 Hz
Format:     MPEG Layer 3
Bit Depth:  32-bit float
Peak RMS:   0.884 (-1.07 dB)
```

---

### 5. `tab` - Renderização de Tablatura ASCII

Lê um áudio ou arquivo de transcrição e renderiza a tablatura com digitação otimizada e símbolos de técnicas no terminal.

```bash
tabforge tab <INPUT>
```

---

### 6. `score` - Exportação de Partitura MusicXML

Gera uma partitura no padrão MusicXML 4.0 com todas as notações de tempo, pausas, compassos e anotações técnicas (`<bend>`, `<hammer-on>`, `<slide>`, etc.).

```bash
tabforge score <INPUT> -o <OUTPUT.musicxml>
```

---

### 7. `midi` - Conversão e Geração MIDI

Converte eventos de áudio / Music IR para arquivos Standard MIDI (.mid) com notas, velocidades e durações precisas.

```bash
tabforge midi <INPUT> -o <OUTPUT.mid>
```

---

### 8. `new` - Inicialização de Novo Projeto

Cria a estrutura de um novo projeto musical com arquivo de configuração padrão.

```bash
tabforge new <PROJECT_NAME>
```

---

## 🎸 Notação de Técnicas na Tablatura

O motor do TabForge reconhece articulações e renderiza símbolos na tablatura ASCII mantendo o alinhamento vertical estrito entre as cordas:

| Símbolo | Técnica | Descrição |
| :--- | :--- | :--- |
| `7` | **Nota Natural** | Traste convencional palhetado. |
| `7b` | **Bend** | Elevação contínua de tom/semitom na corda. |
| `7r` | **Release Bend** | Retorno do bend ao tom natural. |
| `7~` | **Vibrato** | Modulação periódica de frequência ($4-8.5\text{ Hz}$). |
| `5/` | **Slide Up** | Glissando ascendente contínuo. |
| `7\` | **Slide Down** | Glissando descendente contínuo. |
| `h7` | **Hammer-On** | Ligadura ascendente sem palhetada forte. |
| `p5` | **Pull-Off** | Ligadura descendente sem palhetada forte. |
| `7.` | **Palm Mute** | Abafamento com a palma da mão (decaimento rápido). |
| `<12>` | **Harmônico Natural** | Ponto de harmônico com nó senoidal puro. |
| `---` | **Pausa (Rest)** | Silêncio musical preenchido na métrica do compasso. |

#### Exemplo de Saída ASCII:
```text
e |-7b---7~-------------------|
B |-----------h8---p5---------|
G |--------------------5/--7\-|
D |---------------------------|
A |---------------------------|
E |---------------------------|
```

---

## ⚙️ Configuração (`tabforge.toml`)

Você pode customizar o comportamento do motor criando ou editando um arquivo `tabforge.toml` no diretório raiz do projeto:

```toml
# TabForge - Arquivo de Configuração

[transcription]
sample_rate = 44100          # Taxa de amostragem padrão (Hz)
hop_size = 512              # Tamanho do salto de frames DSP
window_size = 2048          # Janela de análise FFT (YIN / STFT)
tolerance_cents = 25.0      # Tolerância de desvio de afinação em cents
min_frequency = 60.0        # Frequência mínima para pitch detection (Hz)
max_frequency = 1200.0      # Frequência máxima para guitarra/baixo (Hz)

[guitar]
strings = 6                 # Número de cordas do instrumento
frets = 24                  # Quantidade total de trastes
tuning = ["E2", "A2", "D3", "G3", "B3", "E4"] # Afinação das cordas
preferred_fret_range = [0, 12] # Faixa preferencial para o otimizador Viterbi

[quantization]
enabled = true              # Ativar quantização rítmica métrica
min_subdivision = "1/16"    # Menor subdivisão rítmica (semicolcheia)
swing = 0.0                 # Quantidade de swing rítmico (0.0 a 1.0)
tolerance_ms = 35.0         # Janela de tolerância para alinhamento temporal

[export]
default_format = "tab"      # Formato padrão de saída (tab, musicxml, midi)
musicxml_version = "4.0"    # Versão do esquema MusicXML
ascii_tab_width = 80        # Largura máxima de caracteres por bloco de tab
```

---

## 🏛️ Arquitetura do Workspace

O projeto é estruturado em crates modulares seguindo Clean Architecture:

```text
tabforge/
├── Cargo.toml                  # Cargo Workspace
├── tabforge.toml               # Configuração padrão do motor
├── README.md                   # Manual e documentação de uso
│
├── crates/
│   ├── tabforge-core/          # Music IR (Song, Track, Measure, Beat, Note, Pitch, Duration)
│   │                           # - Independente de bibliotecas de áudio e MIDI.
│   │                           # - Aritmética temporal racional exata (num-rational).
│   │
│   ├── tabforge-guitar/        # Motor de guitarra e baixo
│   │                           # - Mapeamento de Fretboard e afinações (Standard, Drop, Bass).
│   │                           # - Otimizador de digitação por Programação Dinâmica (Viterbi).
│   │                           # - Dicionário de formatos de acordes.
│   │
│   ├── tabforge-audio/         # DSP e decodificação de áudio
│   │                           # - Symphonia (decodificador universal multi-formato).
│   │                           # - YIN Pitch Tracker acelerado por FFT.
│   │                           # - Spectral Flux Onset & Beat Tracker com log-Gaussian prior.
│   │                           # - Segmentador ADSR e Detector de Articulações de guitarra.
│   │                           # - HPSS e Gerenciador de Modelos Neurais de Stems.
│   │
│   ├── tabforge-midi/          # Processamento MIDI
│   │                           # - Parser e gravador Standard MIDI (midly).
│   │                           # - Conversor bidirecional MIDI <-> Music IR.
│   │
│   ├── tabforge-score/         # Exportadores de notação
│   │                           # - Renderizador de tablatura ASCII com alinhamento vertical.
│   │                           # - Exporter MusicXML 4.0 com anotações técnicas completas.
│   │
│   └── tabforge-cli/           # Interface de Linha de Comando (CLI)
│                               # - Subcomandos: transcribe, stems, models, inspect, tab, score, midi, new.
```

---

## 🧪 Testes Automatizados

Para executar todos os testes unitários e de integração de todo o workspace:

```bash
cargo test --workspace
```

---

## 📄 Licença

Distribuído sob licença MIT / Apache-2.0. Consulte o arquivo `LICENSE` para mais detalhes.
