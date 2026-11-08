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
| `etape-5-terrain.png` | Une boîte 100 × 100 avec une tache de nutriments, un mur gris avec une porte et la colonie à 80 pas. | `cargo run -- lancer scenarios/terrain.txt --graine 1 --sortie <dossier>`, puis copier `apercu.png` |
| `etape-6-chimiotropisme.png` | La colonie du scénario `scenarios/chimiotropisme.txt` (graine 7, 100 pas, force 10) : une tache de nutriments en haut à droite de la spore, vers laquelle les hyphes se penchent ; elles s'enroulent autour d'elle. La graine 7 a été choisie parmi 40 pour que les deux colonies (avec et sans) aient à peu près la même taille ; la mesure sur les 40 graines est dans `MODELE.md`. | `cargo run -- lancer scenarios/chimiotropisme.txt --graine 7 --sortie <dossier>`, puis copier `apercu.png` |
| `etape-6-sans-chimiotropisme.png` | Le même scénario sans chimiotropisme (`scenarios/chimiotropisme-sans.txt`, force 0, graine 7) : la colonie ne se penche pas vers la tache. | `cargo run -- lancer scenarios/chimiotropisme-sans.txt --graine 7 --sortie <dossier>`, puis copier `apercu.png` |
| `etape-7-mur.png` | La colonie du scénario `scenarios/mur.txt` (graine 31, 120 pas, `thigmotropisme 35`) : un mur gris foncé devant la spore, à gauche. Les hyphes qui le touchent de biais le longent, celles qui le frappent de face s'arrêtent et se ramifient contre lui : elles s'accumulent le long du mur. La colonie est environ deux fois plus grosse qu'à angle nul. La graine 31 a été choisie parce que, dans les deux cas, la colonie atteint le mur et passe de l'autre côté ; la mesure sur 40 graines est dans `MODELE.md`. | `cargo run -- lancer scenarios/mur.txt --graine 31 --sortie <dossier>`, puis copier `apercu.png` |
| `etape-7-sans-thigmotropisme.png` | Le même scénario sans thigmotropisme (`scenarios/mur-sans.txt`, angle 0, graine 31) : une pointe qui touche le mur y reste collée, sans le longer ni se ramifier contre lui. | `cargo run -- lancer scenarios/mur-sans.txt --graine 31 --sortie <dossier>`, puis copier `apercu.png` |

Chaque étape de la feuille de route ajoute ses aperçus ici, et ce fichier se met à jour avec elle.
