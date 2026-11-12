//! Mycelium Lab : la simulation, ses mesures et son rendu. Le binaire `mycelium` les lance ; l'étape 0 de
//! `FEUILLE-DE-ROUTE.md` pose les premiers modules.
//!
//! # Le binaire
//!
//! `mycelium lancer <scénario> --graine <n> --sortie <dossier>` lit le scénario, construit l'état avec la
//! graine, puis écrit dans le dossier de sortie un fichier `empreinte.txt`. Ce fichier contient l'empreinte
//! hexadécimale de l'état (64 caractères minuscules) suivie d'un saut de ligne.
//!
//! # Modules
//!
//! - [`alea`] : générateur pseudo-aléatoire déterministe ;
//! - [`grille`] : l'état simulé, son écriture canonique et son empreinte ;
//! - [`scenario`] : la boîte de Petri et sa lecture depuis un texte ;
//! - [`empreinte`] : SHA-256 ;
//! - [`image`] : écrivain PNG et sommes de contrôle ;
//! - [`croissance`] : apex, marche persistante, ramification ;
//! - [`substrat`] : diffusion explicite du substrat ;
//! - [`mesures`] : biomasse et dimension fractale ;
//! - [`calage`] : mesurer la colonie en unités qu'une publication donne (longueur d'hyphe par pointe, taux de croissance,
//!   fenêtre de croissance) — l'étape 12 (M2) : elle mesure, elle ne change rien au noyau de la simulation ;
//! - [`moule`] : donner une forme à l'espace libre (brique, L, disque, anneau, labyrinthe), la développer en lignes
//!   `obstacle` d'un scénario, et mesurer comment la colonie remplit ce moule et en sort — l'étape M1 : elle ne change rien au
//!   noyau de la simulation ;
//! - [`piece`] : mesurer la pièce que la colonie remplit (carte de densité, bilan de nourriture, plus grand morceau
//!   connecté, traversée d'un bord à l'autre) — l'étape M0 : elle mesure, elle ne change rien à la simulation ;
//! - [`systeme`] : le solveur linéaire du calcul de raideur (Cholesky d'une matrice pleine, mise à l'échelle par la
//!   diagonale, détection d'un mécanisme) — l'étape 13 (M3) ;
//! - [`raideur`] : le calcul de raideur d'un réseau de barres articulées, vérifié sur des cas d'école à réponse exacte —
//!   l'étape 13 (M3) : une couche à part, elle ne change rien au noyau de la simulation ;
//! - [`poutres`] : le calcul de raideur d'un réseau de poutres à nœuds rigides (étirement et flexion, trois degrés de
//!   liberté par nœud) — l'étape 14 (M3) : une couche à part aussi ;
//! - [`reseau`] : le réseau de poutres d'une colonie (ses segments, plus un pont par anastomose) et l'essai de poussée qui
//!   en donne la raideur entre deux bords — l'étape 15 (M3) : une couche à part, elle ne change rien au noyau de la
//!   simulation ;
//! - [`tropisme`] : le chimiotropisme (un virage de la pointe vers le côté où il y a plus de nutriments) et le
//!   thigmotropisme (ce qu'une pointe fait à son contact avec un obstacle).

pub mod alea;
pub mod anastomose;
pub mod calage;
pub mod croissance;
pub mod empreinte;
pub mod grille;
pub mod image;
pub mod mesures;
pub mod moule;
pub mod piece;
pub mod poutres;
pub mod raideur;
pub mod releves;
pub mod reseau;
pub mod scenario;
pub mod simulation;
pub mod substrat;
pub mod systeme;
pub mod tropisme;
pub mod vitrine;
pub mod voisinage;
