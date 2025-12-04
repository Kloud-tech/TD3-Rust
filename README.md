# TD3-Rust – Analyseur de logs en CLI

Outil CLI complet pour analyser des fichiers de logs (format `YYYY-MM-DD HH:MM:SS [LEVEL] message`) avec filtres, stats et sorties multiples.

Consignes du TD : voir `td3.md`.

## Fonctionnalités clés
- CLI `clap` avec aide complète (`--help`) et validation (`--top` ≥ 1).
- Parsing regex + datetime (`chrono`), filtres `--errors-only`, `--search`, fenêtres temporelles `--since/--until`.
- Formats de sortie : texte (tables colorées), JSON, CSV. Option `--output FILE`.
- Stats : total, répartition par niveau, top N erreurs, erreurs par heure, taux d’erreurs par heure.
- Performance : mode parallèle auto (>10 MB) ou forcé (`--parallel`), barre de progression `indicatif`, timings `--verbose`.
- Gestion des erreurs : message clair pour fichier introuvable ou aucun résultat après filtres.

## Installation
```bash
cargo build
```

## Exemples d’usage
- Aide : `cargo run -- --help`
- Analyse simple : `cargo run -- sample.log`
- Filtrer erreurs + recherche : `cargo run -- --errors-only --search api sample.log`
- Fenêtre temporelle : `cargo run -- sample.log --since "2024-01-15 10:31:00" --until "2024-01-15 10:32:00"`
- Top 10 en CSV : `cargo run -- --errors-only --top 10 --format csv sample.log > out.csv`
- Forcer le parallèle + timings : `cargo run -- --parallel --verbose sample.log`

## Format de log attendu
```
2024-01-15 10:30:45 [INFO] Application started
```
Niveaux reconnus : INFO, WARNING/WARN, ERROR, DEBUG.

## Tests
```bash
cargo test
```
- Tests unitaires : parsing, filtrage, analyse.
- Tests d’intégration : aide CLI, filtres erreurs+search, filtres temporels.

## Dépendances principales
- clap, regex, chrono, serde/serde_json, prettytable, colored, rayon, indicatif, once_cell
