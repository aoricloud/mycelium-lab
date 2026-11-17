---
name: test-qui-lit-a-lu
description: Un test qui parcourt des fichiers, des lignes ou une liste vérifie qu'il a vu au moins un élément : sinon il passe sans rien vérifier.
---

## What happened
Le 16/11/2026, vague des règles (PR 88) : `tests/regles_du_code.rs` lisait chaque fichier du dépôt et en vérifiait les lignes une à une. Un fichier vidé aurait passé toutes les vérifications sans qu'aucune le voie. La relecture l'a relevé en remarque bloquante ; la correction a ajouté `assert!(texte.lines().next().is_some(), ...)`.

## Why
Une boucle sur zéro élément ne fait aucune assertion : le test reste vert, quel que soit le code. Le même trou s'ouvre quand un motif de chemin ne trouve plus aucun fichier, ou quand une liste lue dans un module (ses exportations, ses lignes) revient vide.

## How it is checked
Fais rougir le test exprès avant de le rendre : vide le fichier lu, ou change le motif pour qu'il ne trouve rien. Le test doit échouer, avec un message qui nomme ce qu'il n'a pas vu.

## Pitfall
Compter ne suffit pas toujours : vérifie aussi que l'élément attendu est là (un fichier précis, une exportation nommée), pas seulement qu'il y en a un.

## Files

## Falls when
Le dépôt lit ses fichiers par une seule fonction commune qui refuse une lecture vide.
