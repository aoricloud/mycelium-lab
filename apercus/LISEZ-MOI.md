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
| `etape-2-animation.png` | La colonie du scénario `scenarios/couplage.txt` (graine 1) aux temps 10, 20, 30, 45, 60 et 80, à 2 pixels par case, en deux lignes de trois, pour voir la colonie pousser et le substrat s'épuiser derrière elle. | `cargo run --example apercus_etape_2` |
| `etape-2-colonie.png` | La colonie du scénario `scenarios/couplage.txt` (graine 1, 80 pas), à 4 pixels par case : ses segments clairs sur un fond dont la luminosité suit le substrat restant. | `cargo run --example apercus_etape_2` |
| `etape-3-colonie.png` | La colonie du modèle de la figure 3 (`voisinage::Colonie`, graine 1, 400 pas, depuis la spore `(0, 0)`), ses segments clairs sur fond sombre, à l'échelle qui la fait tenir dans l'image. | `cargo run --example apercus_etape_3` |
| `etape-3-figure.png` | La figure refaite : le rayon en fonction du temps pour les graines 1 à 5 sur 400 pas (cinq courbes de couleurs distinctes), la droite de régression de la pente moyenne sur la fenêtre `]200, 400]` en blanc, et la droite de pente 1 (la vitesse d'un apex) en pointillés ; des axes simples, sans texte. | `cargo run --example apercus_etape_3` |

Chaque étape de la feuille de route ajoute ses aperçus ici, et ce fichier se met à jour avec elle.
