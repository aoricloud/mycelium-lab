# Mycelium Lab — la feuille de route de la V0

Ce document est le plan de la V0. Ses étapes deviennent des issues, et il change par PR, comme le code. Seules
l'autonomie de Lydite et ses trois limites sont décidées par l'humain ; le reste est proposé, et se corrige en route.

## Ce que c'est

Un moteur de simulation de croissance mycélienne, libre, en Rust, orienté recherche, avec une interface visuelle
claire : un labo virtuel. Il simule la croissance d'un réseau (extension des apex, ramification, anastomose,
translocation des nutriments, tropismes, stress), explore l'espace des paramètres, se calibre sur des données
réelles, en tire des mesures (biomasse, densité, topologie, dimension fractale) et compare des scénarios. Le visuel se
tient entre le scientifique élégant, crédible pour un labo, et l'organique un peu hypnotique, qu'on laisse tourner pour
le plaisir de regarder.

« Outil de recherche » se gagne : sans chercheur qui l'utilise ni donnée réelle, ce serait un très joli jouet. Le
critère sérieux, dès la V0 : reproduire une figure publiée, en nombres. C'est aussi la meilleure épreuve pour Lydite,
parce que cette preuve ne dépend de l'avis d'aucun agent.

## Ce que la V0 est, et ce qu'elle n'est pas

- **Elle est** : une colonie qui pousse en 2D sur une boîte de Petri depuis une spore ; des nutriments qui diffusent sur
  une grille ; une graine fixe, donc même graine, même colonie ; des mesures en nombres ; une figure publiée reproduite.
- **Elle n'est pas encore** : la 3D, des liaisons Python, un salon de discussion, une prépublication, la calibration
  sur des images. Tout cela attend une V0 qui tourne, ou un chercheur qui le demande.

## Les principes

1. **Rust sans dépendance**, comme Lydite : générateur pseudo-aléatoire, écrivain PNG et empreinte écrits dans le
   dépôt. La CI reste simple et rien n'est à auditer.
2. **Déterministe sur toutes les machines** : même graine, même état, sous Linux comme sous Windows. Donc pas de
   `sin`, `cos`, `exp` ou `ln` de `f64`, qui passent par la bibliothèque mathématique du système et peuvent différer
   d'un système à l'autre : des tables de directions ou des versions écrites dans le dépôt. Pas de `HashMap` parcourue
   (son ordre change d'une exécution à l'autre) : des `Vec` ou des `BTreeMap`. Un ordre de mise à jour fixe.
3. **La preuve, c'est l'état et les nombres** : l'empreinte de l'état simulé, par scénario et par graine
   (`references/<scenario>.empreinte`), et des mesures avec une tolérance. Les tests d'acceptation d'une vague sont
   écrits avant elle (voir « Les tests avant les mains »).
4. **Le rendu, c'est pour l'œil** : les aperçus (la colonie, la carte des nutriments, les images d'une animation) sont
   commités dans `apercus/` pour se voir dans le diff de la PR, y compris sur un téléphone, et restent libres
   d'évoluer : ils ne sont pas une preuve. Embellir le rendu ne touche aucune référence.
5. **Un changement voulu de l'état** (un mécanisme ajouté) met à jour l'empreinte de référence ; la PR nomme ce
   fichier, et c'est là que l'humain juge, l'image d'avant et d'après et les mesures côte à côte. Quand l'empreinte ne
   bouge pas, la simulation est identique et seul le rendu a changé.
6. **Chaque mécanisme arrive avec sa référence** publiée (article, figure ou équation) et sa reproduction en nombres.

## Les modules

| Module | Rôle |
|---|---|
| `alea` | générateur pseudo-aléatoire (PCG32 ou xorshift64*), la graine |
| `grille` | la boîte de Petri (un disque sur une grille carrée), ses champs |
| `croissance` | les apex : extension, marche persistante, ramification, anastomose |
| `substrat` | les nutriments : diffusion (schéma explicite, pas de temps stable), consommation, conservation |
| `mesures` | biomasse, nombre de pointes, densité de ramification, longueur par pointe, profil radial, vitesse radiale, dimension fractale par comptage de boîtes, conservation de la masse |
| `scenario` | les paramètres (un fichier texte simple), la graine, la durée |
| `empreinte` | l'empreinte de l'état, sur une écriture canonique |
| `image` | l'écrivain PNG (blocs deflate stockés, CRC32, Adler32) |
| `rendu` | les aperçus et les images d'animation |
| binaire `mycelium` | `mycelium lancer <scénario> --graine <n> --sortie <dossier>` : l'empreinte, les mesures en CSV, les aperçus |

## L'autonomie de Lydite

Lydite mène Mycelium Lab en autonomie totale, dans trois limites décidées par l'humain :
1. la preuve et le mur restent : Lydite rejoue lui-même chaque preuve, et les mains travaillent derrière le mur ;
2. ce seul dépôt : un jeton limité à Mycelium Lab ; les réglages du dépôt (règles, vérifications exigées, Pages) restent
   à l'humain, et Lydite ne peut pas lever ses propres garde-fous (il ne pousse jamais `.github/`, n'a aucun droit
   d'administration) ;
3. seulement les issues de l'humain ou celles de Lydite : sur un dépôt public, le texte d'un inconnu ne devient jamais
   la consigne d'une main.

Le reste est libre : écrire ses issues, choisir l'ordre, fusionner, enchaîner. Proposé, non décidé : l'humain pose
l'étiquette `aurait-refuse` sur une PR fusionnée qu'il aurait refusée ; Lydite la compte comme un raté et ouvre une
issue de correction.

**La fusion, sans une ligne de Lydite** : l'auto-fusion du dépôt est permise, et la règle de `main` exige la CI (Linux
et Windows). `auto-fusion.yml` demande l'auto-fusion des seules PR des branches `lydite/` du dépôt vers `main` (et
`kraken/`, l'ancien nom de Lydite, le temps que finissent les vagues lancées avant le changement), déclenché par
`pull_request` (jamais `pull_request_target`) ; GitHub fusionne quand tout est vert. La même CI refuse
qu'une dépendance ou un `.cargo/` entre.

**Le plan vit dans le dépôt** : ce document, et ses étapes en issues. Tant que Lydite ne prend pas encore ses issues
seul, ses vagues sont lancées d'office, une étape après l'autre.

## Les tests avant les mains

Chaque étape a deux vagues. La première écrit les signatures publiques de l'étape (des corps qui paniquent) et ses
tests d'acceptation, d'après l'issue et sa référence ; elle déclare la fonctionnalité `etape-<n>` dans `Cargo.toml`,
sans l'activer, et chaque test porte `#[cfg_attr(not(feature = "etape-<n>"), ignore)]`. Elle est fusionnée comme les
autres. La seconde remplit les corps et ajoute `etape-<n>` aux fonctionnalités par défaut, ce qui lève les `ignore`
sans toucher aux tests : ses tâches n'ont pas les fichiers de tests dans leurs `fichiers`, et Lydite refuse une main
qui y touche. C'est la meilleure parade d'ImpossibleBench : des tests en lecture seule.

## Les règles que les mains ne lisent pas encore

Une main de Lydite ne reçoit pas encore les consignes du dépôt (`AGENTS.md`, `CLAUDE.md`). Chaque issue redit donc les
règles qui comptent pour son étape : le déterminisme (ni `sin` ni `exp` du système, ni `HashMap` parcourue), les tests
qu'on ne touche pas, le « sans dépendance ». La CI sous Linux et sous Windows attrape un état qui différerait d'un
système à l'autre : l'empreinte de référence n'y serait plus la même.

## Les vagues

Chaque vague part par Lydite sur une issue ou sur un plan écrit par la session pilote et accepté par André (`lydite
lancer nautile --plan <fichier> --pr main`), sous le profil `nautile` : le mur exigé, DeepSeek Flash aux mains (le
profil `mycelium`, Sonnet, reste le recours). GitHub fusionne ce qui est vert. La preuve :
`cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check`.

| Étape | Tâches (en parallèle dans une vague) | Tests d'acceptation | Aperçu |
|---|---|---|---|
| 0, le socle | (a) `alea`, `grille`, `scenario`, `empreinte` et le binaire qui lance un scénario vide ; (b) `image` et un premier aperçu | même graine, même empreinte ; deux graines, deux empreintes ; PNG relu octet par octet (signature, CRC, Adler) | la boîte vide |
| 1, trois branches | (a) croissance : apex, pas, marche persistante, ramification ; (b) substrat : diffusion, bords sans flux, conservation ; (c) mesures, sur des images synthétiques | (a) longueur d'un apex = pas × temps, intervalle de ramification moyen à quelques pour cent du paramètre, isotropie sur plusieurs graines ; (b) masse conservée, variance d'un pic qui croît comme 2Dt, symétrie ; (c) carré plein → 2, segment → 1, tapis de Sierpinski → ln 8 / ln 3 ≈ 1,893 | un arbre d'hyphes ; un pic qui s'étale ; les images synthétiques |
| 2, le couplage | (a) la colonie : les apex consomment, l'extension dépend du substrat, une spore au centre ; (b) l'anastomose ; (c) les mesures branchées sur la simulation, en CSV par pas de temps | substrat + biomasse conservés ; rayon linéaire après une latence (R² au-dessus d'un seuil sur la fenêtre tardive) ; anastomoses nulles quand l'option est coupée, reproductibles sinon ; empreintes de référence | la colonie, et son animation |
| 3, la référence | (a) reproduire la figure choisie (voir plus bas), avec une tolérance ; (b) `MODELE.md` : équations, hypothèses, limites, d'où vient chaque valeur | la figure en nombres | la figure refaite, à côté de l'originale |
| 4, la vitrine | une page web où la simulation tourne en direct : cible `wasm32-unknown-unknown` sans dépendance, quelques fonctions exportées, un canvas en JavaScript | la page charge et l'empreinte d'une graine est la même qu'en natif | la page elle-même |

Chaque ligne est une étape, en deux vagues : ses tests d'abord, son code ensuite (voir « Les tests avant les mains »).
La vitrine se publie sur GitHub Pages une fois que l'humain l'a ouvert, son geste, une fois ; ensuite chaque fusion la
republie. Elle donne aussi la matière d'une vidéo.

## Après la V0, dans l'ordre, chacune avec sa référence

1. Les tropismes (chimiotropisme vers les nutriments, thigmotropisme contre les obstacles) : décidée le 08/11, voir la section suivante.
2. Les milieux hétérogènes et les obstacles : la lignée de Boswell (2003, 2007), Hopkins et Boswell (2012).
3. Les stress simples.
4. Le balayage de paramètres en local (un mode par lots).
5. La documentation scientifique du modèle, complète.

Attendent une V0 qui tourne, ou un chercheur : la 3D, Python, un salon, une prépublication, la calibration sur images,
la compétition entre espèces.

## L'étape 1 d'après la V0 : les tropismes

Décidée par André le 08/11. Les pointes de la colonie ne font aujourd'hui que marcher au hasard : elles apprennent à
choisir leur chemin. Le **chimiotropisme** les penche vers le côté où il y a plus de nutriments ; le **thigmotropisme**
leur fait longer un obstacle, ou se ramifier contre lui. Les tropismes entrent dans le couplage
(`simulation::Simulation`, celui de la vitrine), pas dans le modèle de voisinage de la figure 3.

**Le scénario** gagne des lignes : `source x y rayon quantité` (une tache de nutriments), `obstacle x0 y0 x1 y1` (un
rectangle de cases infranchissables, sans flux de substrat à travers), `chimiotropisme force` et `thigmotropisme angle`
(éteints par défaut). Sans ces lignes, ou à force zéro, le moteur est celui de la V0 bit à bit :
`references/couplage-graine-1.empreinte` ne change pas.

**Les références**, à vérifier sur l'article avant de geler un test :
- thigmotropisme : M. Held, O. Kašpar, C. Edwards et D. V. Nicolau, *PNAS* 116(27):13543–13552 (2019), des hyphes de
  *Neurospora crassa* dans des structures microfluidiques : une hyphe qui touche un obstacle sous un angle inférieur à
  35° par rapport à sa surface le longe ; au-delà, elle perd sa mémoire de direction et se ramifie contre lui ; elle
  retrouve sa direction à environ 20° près. D'autres : K. L. Hanson et coll., *Small* 2:1212–1220 (2006) ; E. Asenova
  et coll., *IEEE Trans. NanoBioscience* 15:613–618 (2016) ;
- chimiotropisme : D. Turrà et coll., *Nature* 527:521–524 (2015) ; S. Hopkins et G. P. Boswell, *Fungal Ecology*
  5:124–136 (2012). Une référence surtout qualitative : aucune figure chiffrée à refaire n'a été trouvée à ce jour, et
  `MODELE.md` le dira. Les vérifications sont alors indépendantes de tout réglage : à force zéro, le moteur de la V0
  inchangé ; une pointe seule dans un gradient constant, dont l'orientation moyenne est égale à une valeur calculée
  exactement.

**Les étapes**, deux vagues chacune, les tests d'abord : T0, le terrain (la tache et l'obstacle dans le scénario, le
substrat sans flux à travers un obstacle) ; T1, le chimiotropisme ; T2, les obstacles et le thigmotropisme ; T4,
la vitrine avec un choix de scénario. T3, un labyrinthe (le temps de sortie de la colonie, comparé à une recherche
en profondeur et à A*), est repliée dans M1, le moule, de la direction décrite plus bas.

**Où on en est** : T0 (le terrain), T1 (le chimiotropisme), T2 (les obstacles et le thigmotropisme), T4 (la vitrine avec un choix de scénario : six scénarios, les murs dessinés, une adresse `#scenario=…&graine=…` qui se partage), M0 (mesurer la pièce : une carte de densité, un seuil, le bilan de nourriture, le plus grand morceau connecté, la traversée d'un bord à l'autre), M1 (le moule : cinq formes dont le labyrinthe de T3, une ligne `moule` développée en obstacles, la courbe de remplissage, le temps de sortie et trois recherches de chemin, sans changer le noyau) et l'étape 11 (la carte de densité de M0 sur la page, derrière une case « Densité », du violet au jaune) sont faits. Le chimiotropisme, le thigmotropisme, la pièce, le moule et la carte de densité sur la page sont décrits, avec leurs limites, dans les cinq dernières parties de `MODELE.md`. La suite est la direction décrite ci-dessous, M2 d'abord.

## La direction d'après T2 : du procédé à la structure (M0 à M5)

Décidée par André le 08/11, d'après l'étude du matériau (`etude-mycelium-materiau-synthese.md`). Le banc devient un
atelier de moulage numérique : on choisit une forme (des obstacles durs), on place la spore et la nourriture, la
colonie remplit la forme, et l'on mesure la pièce obtenue. Il montre ce que le moule et la nourriture font au réseau ;
**il ne prédit pas la résistance d'un vrai matériau**. Il reste en 2D, à l'échelle de l'hyphe (de petits moules, de
l'ordre du millimètre) ; la 3D viendra plus tard, en chantier à part. Les nouveaux réglages (diamètre d'hyphe, raideur
de la paroi, ramification selon la place) vont dans un profil séparé : la V0 et sa figure 3 restent intactes.

**L'ordre** : T4, la vitrine des scénarios (faite) ; M0, mesurer la pièce (faite : une carte de densité, un
seuil, le bilan de nourriture, le plus grand morceau connecté, la traversée, sans changer l'état ; sa carte se voit sur la
page depuis l'étape 11, case « Densité ») ; M1, le
moule (faite : brique, L, disque, anneau et le labyrinthe de T3, un préprocesseur de scénario, la courbe de remplissage,
le temps de sortie vers un rectangle de cases, le plus court chemin, A* et la profondeur d'abord, sans changer le noyau)
; M2, une croissance calée sur des chiffres publiés ; M3, le calcul de raideur d'un réseau de barres, dans une couche
à part, des cas d'école à réponse exacte d'abord, la flexion ensuite ; M4, la raideur selon la densité ; M5,
facultatif, des particules et la comparaison à un classement publié (Rigobello et Ayres, 2022). Aucun chiffre marqué «
à vérifier » par l'étude n'entre dans un test sans relecture à la source.

## La référence de la V0

D'après la réimpression de l'article, le modèle de Meškauskas, Fricker et Moore (2004) est **en 3D** ; leurs colonies
« de boîte de Petri » sont du 3D tenu en couche mince (4 unités) par un tropisme horizontal (constante 0,002), sans
vrai 2D. L'article n'a aucun tableau, et la plupart de ses figures sont des images. Une seule figure chiffrée se refait
en 2D : la **figure 3**, où le rayon de la colonie croît linéairement avec le temps, avec une pente d'environ 0,86 fois
la vitesse d'un apex (« The slope of the line in both cases in Fig. 3 is about 0.86 ») ; l'écart à 1 vient des apex
qui ne partent pas droit vers l'extérieur. Les paramètres donnés : la vitesse d'un apex (1), l'autotropisme négatif
(facteur de persistance k = 0,1), la ramification selon la densité (seuil 0,06, constante d'ajustement lc = 20 du poids
d'une section, 40 % par itération) ; le noyau n'a pas de coupure dans l'article. Les formules du modèle sont dans son
compagnon, Meškauskas, McNulty et Moore (2004), Mycological Research 108:341–353 (équations 3, 4, 11 et 13) ; manquent
l'angle de ramification et le nombre d'apex au départ ; le programme Java est cité, pas son code. Les figures 6 et 7
dépendent du passage du 3D au plat et ne se refont pas en 2D. L'article ne donne aucune mesure de vraies colonies.

Pour la V0, donc :
- **La cible** : la figure 3, en nombres : une croissance linéaire du rayon, pas exponentielle (R² d'au moins 0,995),
  une pente moyenne sous 1 et dans l'intervalle `[0,80 ; 0,97]` (la valeur publiée, 0,86, est dans l'intervalle mais
  n'est pas la cible ; l'écart mesuré se publie dans `MODELE.md`), et une vitesse radiale constante (Edelstein, 1982),
  avec les paramètres publiés ; ce qui manque est choisi et écrit dans `MODELE.md`. C'est une reproduction dans
  l'esprit de la figure, en 2D, pas sa copie exacte, et `MODELE.md` le dit.
- **Une seconde vérification**, qui ne dépend d'aucun réglage : la vitesse radiale constante prédite par Edelstein
  (1982).
- **Les mesures de vraies colonies** viennent d'ailleurs (par exemple une dimension fractale d'environ 1,94 pour une
  colonie entière d'Ashbya gossypii, Ritz et Crawford 1990) et servent de garde-fous larges, pas de cible.
- **Les mesures unifiées** : biomasse (longueur totale), nombre de pointes, densité de ramification, longueur par
  pointe (l'« unité de croissance hyphale » de l'article), dimension fractale, rayon et vitesse radiale, profil radial,
  conservation du substrat et de la biomasse.
- **Les nutriments** de la V0 ne viennent pas de cet article : ils suivent la lignée de Boswell, à lire avant la
  vague 2.

## Les rejeux, pour Lydite

Chaque milestone de Lydite se mesure en rejouant des étapes déjà faites, à base et issue gelées. Un rejeu part d'un
dépôt neuf qui ne porte que l'arbre de la base, sans l'historique ni les branches de la suite : sinon une main lirait la
solution dans git (`git log --all`, les objets du dépôt). Les issues y sont recopiées. Des issues piégées, dont le test
contredit le texte, ne servent qu'aux rejeux.

## Les gestes de l'humain

Faits, une fois : le dépôt, public, sous MIT ; un jeton limité à lui ; l'auto-fusion et la règle de `main` ; la
première PR (la CI et `auto-fusion.yml`) ; Pages, ouvert : la vitrine est en ligne,
https://aoricloud.github.io/mycelium-lab/. Plus aucune fusion ensuite ; son œil revient après coup.

## Ce qui existe déjà (non revérifié)

mycorust (Rust, MIT, plutôt ludique) ; pycelium (Python, le plus proche du positionnement, sans licence) ; Mycemulator
(Python, une équipe étudiante, 2020). La place « rigoureux, reproductible, beau, en Rust » est libre.

## Sources

- Meškauskas, Fricker et Moore (2004), Mycological Research 108(11):1241–1256 ; réimpression :
  https://davidmoore.org.uk/cyberweb/images_2016/Manuscripts/Meskauskas_etal_simulating_coloniesNS02.pdf
- Boswell et al. (2003), Bull. Math. Biol. ; Boswell et al. (2007), DOI 10.1007/s11538-005-9056-6 ; Hopkins et
  Boswell (2012), Fungal Ecology ; Edelstein (1982), J. Theor. Biol. 98 ; https://arxiv.org/pdf/q-bio/0402004.
