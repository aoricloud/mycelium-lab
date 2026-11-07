# Les aperçus

Les aperçus sont pour l'œil : commités dans `apercus/`, ils se voient dans le diff d'une PR, y compris sur un
téléphone. Ils restent libres d'évoluer et ne sont pas une preuve : la preuve, ce sont l'état simulé et des nombres
(voir `FEUILLE-DE-ROUTE.md`).

| Image | Ce qu'elle montre | La réécrire |
|---|---|---|
| `etape-0-boite-vide.png` | La boîte de Petri vide vue de dessus, écrite par le binaire `mycelium lancer` sur le scénario `boite 64 48` ; la commande écrit `<dossier>/apercu.png`, l'image commitée en est la copie. | `cargo run -- lancer <fichier contenant la ligne « boite 64 48 »> --graine 1 --sortie <dossier>` |
| `etape-1-arbre.png` | Une colonie d'hyphes partie d'une spore, ses segments clairs sur fond sombre. | `cargo run --example apercus_etape_1` |
| `etape-1-pic.png` | Un pic de substrat qui s'étale, en niveaux de gris. | `cargo run --example apercus_etape_1` |
| `etape-1-sierpinski.png` | Le tapis de Sierpinski de niveau 5, dont la dimension fractale mesurée vaut ln 8 / ln 3 ≈ 1,89. | `cargo run --example apercus_etape_1` |

Chaque étape de la feuille de route ajoute ses aperçus ici, et ce fichier se met à jour avec elle.
