# TD3 – Rust : CLI d'analyse de logs

## Comment exécuter rapidement
- Lancer l'aide : `cargo run -- --help`
- Analyser un fichier : `cargo run -- sample.log`
- Erreurs uniquement + recherche : `cargo run -- --errors-only --search api sample.log`

## Partie 1 – CLI (Arg parsing)
- Ajouter la dépendance `clap = { version = "4.5.51", features = ["derive"] }`.
- Lancer le programme avec `--help` pour observer l'aide générée.
- Ajouter un argument `--top <N>` (ex: `--top 10`).
- Ajouter un argument `--search <TEXT>` pour filtrer les logs contenant un texte.
- Tester avec des arguments invalides et observer les erreurs.

## Partie 2 – I/O & Parsing
- Ajouter `regex = "1.12.2"`.
- Lire un fichier log avec `BufReader`, parser ligne par ligne via regex (timestamp, niveau, message).
- Exercice : intégrer `read_logs()` à la CLI, implémenter `--errors-only`, `--search`, gérer “fichier introuvable”, compter les entrées par niveau.
- Fichier exemple `sample.log` fourni.

## Partie 3 – Sorties structurées & Analyse
- Concepts : `serde`, `prettytable-rs`, CSV, analyse des patterns (top erreurs, tendances temporelles).
- Implémenter les sorties texte/JSON/CSV, top N erreurs, regroupement des erreurs par heure.
- Exercice : `--top N` dynamique, analyse horaire, sortie CSV complète, `--output FILE`, coloriser le tableau (erreurs en rouge, warnings en jaune).

## Partie 4 – Performance & Parallélisme
- Concepts : `rayon` pour paralléliser le parsing/analyse, seuil de taille pour activer le parallèle, mesure des temps.
- Exercice : générer un gros fichier (100k lignes), comparer séquentiel vs parallèle, ajouter `--parallel`, optimiser la compilation de la regex (`once_cell`/`lazy_static`), afficher une progression sur gros fichiers (`indicatif`).

## Livrable attendu
- CLI professionnelle (aide, validation), parsing efficace sur gros fichiers, filtres riches, stats (par niveau, top erreurs, patterns temporels), sorties texte/JSON/CSV, parallélisme optionnel, gestion d'erreurs propre.

## Ressources
- Clap, regex, serde, rayon docs
- Rust CLI Book: https://rust-cli.github.io/book/
