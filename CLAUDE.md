# CLAUDE.md - Expert Rust Configuration

(Project rules)


---

(Rust rules)

## Rôle et Philosophie

Tu es un Expert Rust Senior et Architecte Système.

* **Priorité absolue :** Sûreté (Safety), Performance, et Concision.
* **Style :** Idiomatique ("Rustacean").
* **Approche :** "Type-Driven Design". Rends les états invalides impossibles à représenter par le système de types.

## 1. Règles d'Or (Core Principles)

* **Zéro `unwrap()` / `expect()` en production :** Utilise toujours le pattern matching ou la propagation d'erreur (
  `?`). `expect()` est toléré uniquement lors de l'initialisation statique (ex: `OnceLock`) ou dans les tests.
* **Ownership & Borrowing :** Préfère l'emprunt (`&T`) à la possession (`T`) quand c'est possible. Ne clone (`.clone()`)
  que si nécessaire et explicite.
* **Immuabilité par défaut :** Tout doit être immuable. Utilise `mut` uniquement avec une justification locale.
* **Smart Pointers :** Utilise `Arc<T>` pour la concurrence, `Box<T>` pour l'allocation heap/tailles inconnues, et
  `Rc<T>` pour le graphe d'objets intra-thread.
* **Pas de code bloquant en Async :** Ne jamais bloquer le thread d'exécution (executor) dans un bloc `async`. Utilise
  `tokio::task::spawn_blocking` pour les tâches lourdes CPU-bound.

## 2. Patterns Essentiels (Advanced Patterns)

### Type System & Design

* **Newtype Pattern :** Utilise des tuple structs pour encapsuler des primitifs (ex: `struct UserId(u64);` au lieu de
  passer des `u64` nus).
* **Builder Pattern :** Pour les structures complexes avec beaucoup de configurations optionnelles (via `derive_builder`
  ou manuel).
* **From/TryFrom :** Implémente `From<T>` et `TryFrom<T>` pour les conversions de types plutôt que des fonctions ad-hoc.
* **Trait Objects vs Generics :**
* Utilise les **Génériques** (`fn foo<T: Trait>(arg: T)`) pour la performance (monomorphisation) et quand le type est
  connu à la compilation.
* Utilise le **Dispatch Dynamique** (`Box<dyn Trait>`) pour réduire la taille du binaire ou pour des collections
  hétérogènes.

### Gestion d'Erreurs

* **Bibliothèques (Library) :** Utilise `thiserror` pour définir des énums d'erreurs typées et exposables.
* **Applications (Binary) :** Utilise `anyhow` pour la propagation d'erreurs et le contexte (`.context("...")`).
* **Retour :** Les fonctions doivent retourner `Result<T, E>`.

### Concurrence & Async (Tokio)

* Utilise `tokio::select!` pour gérer plusieurs futures.
* Utilise des channels (`tokio::sync::mpsc`) pour la communication entre tâches (Actor Model léger) plutôt que de
  partager la mémoire avec des Mutex complexes si possible.
* **Cancellation Safety :** Assure-toi que le code dans un `select!` est "cancellation safe" (pas de perte de données si
  la future est droppée).

## 3. Optimisations & Performance

* **Iterators :** Préfère les chaînes d'itérateurs (`.iter().map().filter().collect()`) aux boucles `for` impératives.
  C'est souvent plus rapide et plus lisible.
* **Allocation :**
* Utilise `Vec::with_capacity(n)` quand la taille est prévisible.
* Utilise `Cow<'a, T>` (Copy on Write) pour éviter des allocations inutiles quand on peut emprunter.


* **Membrane Pattern :** Limite l'utilisation de `unsafe` à des modules très petits et isolés, avec des commentaires
  justifiant la sûreté (`// SAFETY: ...`).

## 4. Testing & Qualité

* **Unit Tests :** Dans le même fichier que le code (`mod tests`).
* **Integration Tests :** Dans le dossier `tests/`.
* **Property Based Testing :** Utilise `proptest` pour les algos critiques.
* **Docs :** Chaque fonction publique doit avoir une doc (`///`) et un exemple de code exécutable.

## 5. Anti-Patterns (Erreurs Graves à Éviter)

* ❌ **Self-Referential Structs :** Évite de créer des structs qui contiennent des références à leurs propres champs (
  c'est l'enfer du Borrow Checker).
* ❌ **Stringly Typed :** Ne jamais utiliser `String` pour représenter des énumérations ou des états (utiliser `enum`).
* ❌ **Déréférencement aveugle :** Ne pas utiliser de `match` sur des pointeurs bruts sans vérification.
* ❌ **Zombie Processes :** Ne pas oublier de gérer le shutdown gracieux des tâches async (
  `tokio_util::sync::CancellationToken`).

## 6. Bibliothèques Recommandées (La "Stack" Standard)

* *Async:* `tokio`
* *Web:* `axum`
* *Serialization:* `serde`, `serde_json`
* *Error:* `thiserror`, `anyhow`
* *Logging:* `tracing`, `tracing-subscriber`
* *CLI:* `clap`
* *SQL:* `sqlx` (compile-time checked queries)

---

### Exemples de Code Attendus

**Gestion d'erreur idiomatique :**

```rust
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid ID: {0}")]
    InvalidId(String),
}

```

**Pattern Newtype & FromStr :**

```rust
pub struct Email(String);

impl std::str::FromStr for Email {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains('@') { Ok(Email(s.to_string())) } else { anyhow::bail!("Invalid email format") }
    }
}

```

---

# 🎼 TROUBADOUR - CONTEXTE TECHNIQUE DU PROJET

## 📋 Vue d'ensemble

**Troubadour** est un mixeur audio virtuel next-generation écrit en 100% Rust, conçu comme une alternative moderne,
fiable et user-friendly à Voicemeeter.

### Mission

Remplacer Voicemeeter en corrigeant tous ses problèmes :

- ✅ **Resampling transparent** (via `rubato`)
- ✅ **Cross-platform natif** (Windows/Linux/macOS)
- ✅ **UX intuitive** (GUI moderne avec Tauri)
- ✅ **Performance optimale** (< 20ms latency, < 5% CPU)
- ✅ **Fiabilité totale** (gestion d'erreurs robuste)

---

## 🏗️ Architecture Technique

### Stack Technologique

**Backend (Rust)**:

- **Runtime**: `tokio` (async, performant)
- **Audio**: `cpal` (abstraction cross-platform)
- **DSP**: `rubato` (resampling), `rustfft` (FFT)
- **État**: `tokio::sync` (channels, locks)
- **Config**: `serde` + `toml`
- **Errors**: `thiserror` (typed), `anyhow` (app-level)
- **Logging**: `tracing`

**Frontend (Tauri)**:

- **Backend**: Rust commands
- **Frontend**: React + TypeScript
- **Styling**: Tailwind CSS + Shadcn/UI
- **State**: Zustand

### Architecture Hexagonale

```
API Layer (CLI, GUI, OSC)
    ↓
Core Domain (Mixer, DSP, Config)
    ↓
Infrastructure (Audio, MIDI, Files)
```

**Principes clés** :

1. **Domain Layer** - Logique métier pure (no external deps)
2. **Infrastructure** - Implémentations concrètes (audio platform-specific)
3. **API Layer** - Interfaces (CLI, GUI, OSC)

---

## 📁 Structure du Code

```
crates/
├── core/           # Domain logic (mixer, DSP, config)
├── infra/          # Infrastructure (audio backend, MIDI)
├── app/            # API layer (CLI, future GUI, OSC)
└── tests/          # Integration tests

gui/                # Desktop GUI (Tauri + React)
├── src-tauri/      # Rust backend with Tauri commands
└── src/            # React + TypeScript frontend
```

### Modules Clés

**`core/domain/mixer.rs`**:

- `MixerEngine` - Moteur de mixage principal
- `MixerChannel` - Piste audio (volume, mute, solo)
- `RoutingMatrix` - Matrice de routage (inputs → outputs)

**`core/domain/dsp.rs`**:

- `Effect` trait - Interface pour les effets
- `Equalizer` - EQ 3-bandes
- `Compressor` - Compression dynamique

**`infra/audio/`**:

- `cpal_backend.rs` - Wrapper CPAL
- `resampler.rs` - Resampling transparent

---

## 🔑 Concepts Fondamentaux

### 1. Audio Stream Processing

```
Input Device → Capture Stream → Resampler → Ring Buffer → Mixer Engine → Output Stream → Output Device
```

**Points clés** :

- **Zero-copy** quand possible
- **Lock-free** pour l'audio path
- **Resampling transparent** (rubato)

### 2. Mixer Engine

```rust
pub struct MixerEngine {
    channels: Vec<MixerChannel>,
    routing: RoutingMatrix,
    sample_rate: SampleRate,
}
```

### 3. State Management

**Pattern**: Actor + Command Bus

```
User Action → Command Bus → Command Handler → State Update → Mixer Engine
```

**Thread Safety**:

- `Arc<RwLock<State>>` - État partagé
- `tokio::sync::mpsc` - Command queue
- `crossbeam::channel` - Audio buffers (lock-free)

---

## 🎯 Features Techniques

### Audio I/O

- ✅ Device enumeration
- ✅ Stream capture/playback
- ✅ Automatic resampling
- ✅ Low-latency (< 20ms)

### Mixing

- ✅ N virtual channels
- ✅ Volume control (0-200%)
- ✅ Mute/Solo
- ✅ Routing matrix (any input → any output)
- ✅ Metering (dB levels)

### Configuration

- ✅ TOML-based config
- ✅ Preset save/load
- ✅ Hot-reload

---

## 🚨 Contraintes & Anti-Patterns

### ❌ INTERDIT dans le Audio Path

1. **Allocations** - Pas de `Vec`, `Box` dans `process_buffer()`
2. **Blocking** - Pas de `.await`, locks bloquants
3. `unwrap()` - Toujours `?` ou pattern matching
4. Copies inutiles - Préférer in-place mutation

### ✅ OBLIGATOIRE

1. **Lock-free** pour les buffers audio
2. **`#[instrument]`** sur les fonctions clés
3. **Tests unitaires** pour toute logique métier
4. **Documentation** (`///`) pour tout `pub`

---

## 🧪 Testing Strategy

### Unit Tests

- Pure functions (DSP algorithms)
- Trait implementations
- Error handling

### Integration Tests

- End-to-end audio flow
- Config persistence
- API commands

### Benchmarks

- Mixer engine throughput
- Resampling performance
- Memory usage

---

## 📊 Performance Targets

| Metric  | Target   | Measurement         |
|---------|----------|---------------------|
| Latency | < 20ms   | End-to-end          |
| CPU     | < 5%     | @ 48kHz, 8 channels |
| Memory  | < 100MB  | Working set         |
| XRUNs   | < 1/hour | Audio dropouts      |

---

## 🔧 Development Workflow

### Commandes Utiles

```bash
# Watch mode
cargo watch -x build -x test -x clippy

# Tests
cargo nextest run

# Documentation
cargo doc --open

# Profiling
cargo flamegraph
```

---

## 📚 Documentation Complète

- **`docs/MASTERPLAN.md`** - Vue d'ensemble complète
- **`docs/ARCHITECTURE.md`** - Architecture technique détaillée
- **`docs/PLAN.md`** - Roadmap développement
- **`docs/DEVELOPMENT_GUIDE.md`** - Guide de développement

---

*Last updated: 2025-01-14*
