# Roadmap

> **Objectif v0.5.0 :** Un mixer audio virtuel 100% fonctionnel, prêt pour les utilisateurs.
> Les versions au-delà de 0.5.0 sont des optimisations et features bonus.
>
> Inspirations : Voicemeeter (puissance), SteelSeries Sonar (UX moderne).

---

## v0.1.0 — Foundation

> Workspace Rust fonctionnel, audio qui passe du point A au point B.

- [x] **Workspace setup** : `troubadour-core`, `troubadour-ui`, `troubadour-shared`
- [x] **Device enumeration** : lister tous les périphériques audio du système (entrées/sorties)
- [x] **Audio passthrough** : capturer un device d'entrée et le router vers un device de sortie
- [x] **Gestion du sample rate** : 44.1kHz, 48kHz, 96kHz avec conversion automatique (rubato)
- [x] **Buffer size configurable** : 64, 128, 256, 512 samples
- [x] **UI skeleton** : fenêtre Dioxus desktop avec liste des devices détectés
- [x] **IPC foundation** : communication UI ↔ Core via crossbeam channels
- [x] **CI** : GitHub Actions (cargo check, clippy, test, fmt) — cross-platform (Win/Mac/Linux)

## v0.2.0 — Mixing Core

> Multi-canaux, routing flexible, contrôles de base. Le coeur du mixer.

- [x] **Canaux virtuels** : créer des canaux d'entrée et de sortie nommés (Mic, Desktop, Browser, Music, Discord...)
- [x] **Routing matrix** : connecter n'importe quelle entrée vers n'importe quelle sortie (N:N)
- [x] **Volume par canal** : gain indépendant par canal (0.0 → 1.0+, avec boost)
- [x] **Mute / Solo** : par canal, avec logique solo exclusive/additive
- [x] **Pan stéréo** : gauche/droite par canal
- [x] **VU-meters temps réel** : niveaux audio affichés en temps réel dans l'UI (peak + RMS)
- [x] **UI Mixer** : faders verticaux, boutons mute/solo, VU-meters animés — Tailwind CSS v4
- [x] **UI Routing matrix** : grille visuelle entrées × sorties (toggle on/off)
- [ ] **Hot-plug** : détection branchement/débranchement de devices en temps réel (reporté v0.4)

## v0.3.0 — DSP & Effects

> Traitement audio professionnel par canal. Qualité studio.

- [ ] **EQ paramétrique** : 3-5 bandes par canal (low shelf, peaking, high shelf)
- [ ] **Compresseur** : threshold, ratio, attack, release, makeup gain
- [ ] **Noise gate** : threshold, attack, release (essentiel pour les micros)
- [ ] **Limiter** : protection contre le clipping sur les sorties
- [ ] **Effects chain** : ordre des effets configurable par canal (drag & drop)
- [ ] **Bypass** : activer/désactiver chaque effet individuellement
- [ ] **UI EQ** : visualisation courbe fréquentielle interactive
- [ ] **UI Compresseur/Gate** : visualisation gain reduction en temps réel
- [ ] **Presets d'effets** : sauvegarder/charger des configurations d'effets

## v0.4.0 — Interface Complète

> UI moderne à la SteelSeries Sonar. Configuration intuitive. Presets globaux.

- [ ] **Design system** : thème sombre moderne, composants réutilisables (knobs, faders, toggles, meters)
- [ ] **Vue Mixer** : interface principale avec tous les canaux, inspirée Sonar
- [ ] **Vue Routing** : matrice de routage visuelle avec câbles ou grille interactive
- [ ] **Vue Device** : configuration des périphériques, sample rate, buffer size
- [ ] **Vue Effects** : chaîne d'effets par canal avec visualisations
- [ ] **Profils globaux** : sauvegarder/charger la configuration complète (routing + volumes + effets)
- [ ] **Profils rapides** : switch instantané entre profils (Gaming, Streaming, Music, Meeting)
- [ ] **System tray** : minimiser dans la barre système, accès rapide
- [ ] **Raccourcis clavier** : mute/unmute, switch profil, volume up/down
- [ ] **Démarrage auto** : option lancement au démarrage du système
- [ ] **Onboarding** : assistant première configuration (détection devices, profil suggéré)

## v0.5.0 — Production Ready

> Version utilisable au quotidien. Remplace Voicemeeter.

- [ ] **Virtual audio devices** : driver audio virtuel visible par le système (WDM/WASAPI sur Windows)
- [ ] **Per-app routing** : router les applications individuellement vers des canaux (comme Sonar)
- [ ] **Multi-output** : envoyer vers plusieurs sorties simultanément (casque + enceintes)
- [ ] **Monitoring** : écouter un canal spécifique sur un device dédié
- [ ] **Recording** : enregistrer n'importe quel canal ou bus en WAV/FLAC
- [ ] **ASIO support** : compatibilité interfaces audio pro (faible latence)
- [ ] **Stabilité** : gestion gracieuse des erreurs audio, reconnexion automatique
- [ ] **Performance** : < 1% CPU idle, < 50 MB RAM, latence < 5ms
- [ ] **Installateur** : MSI (Windows), DMG (macOS), AppImage/deb (Linux)
- [ ] **Documentation utilisateur** : guide complet d'utilisation
- [ ] **Tests** : couverture ≥ 80% sur troubadour-core

---

## Post v0.5.0 — Évolutions futures

> Optimisations, features avancées, communauté.

- [ ] **Plugin system** : charger des VST3/CLAP comme effets
- [ ] **Spectrogramme** : visualisation fréquentielle temps réel
- [ ] **Surround** : support 5.1 / 7.1
- [ ] **MIDI control** : mapper des contrôleurs MIDI physiques aux faders/knobs
- [ ] **Thèmes** : thèmes custom, éditeur de thème
- [ ] **API locale** : contrôle via REST/WebSocket (intégration Stream Deck, scripts)
- [ ] **Auto-ducking** : baisser automatiquement la musique quand le micro capte
- [ ] **Spatial audio** : positionnement 3D des sources
- [ ] **Cloud sync** : synchroniser les profils entre machines
- [ ] **Marketplace** : partager des presets d'effets communautaires
