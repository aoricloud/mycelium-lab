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
//! - [`tropisme`] : le chimiotropisme (un virage de la pointe vers le côté où il y a plus de nutriments) et le
//!   thigmotropisme (ce qu'une pointe fait à son contact avec un obstacle).

pub mod alea;
pub mod anastomose;
pub mod croissance;
pub mod empreinte;
pub mod grille;
pub mod image;
pub mod mesures;
pub mod releves;
pub mod scenario;
pub mod simulation;
pub mod substrat;
pub mod tropisme;
pub mod vitrine;
pub mod voisinage;
