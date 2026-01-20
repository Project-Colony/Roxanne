# Overview – Vision produit

## Vision
Roxanne est un éditeur de texte moderne, rapide et extensible, centré sur la
simplicité d’usage et la performance. Le produit vise à offrir une expérience
fluide pour l’édition quotidienne tout en restant léger et modulaire.

## Portée (scope)
- Édition locale de fichiers texte.
- Expérience clavier prioritaire, souris en support.
- Extensibilité via un système de plugins.
- Configuration riche (thèmes, raccourcis, préférences par workspace).

## Hors scope (à ce stade)
- Collaboration en temps réel.
- Édition de documents riches (WYSIWYG).
- IDE complet (debugger intégré, build system avancé).
- Version web ou mobile native.

## Utilisateurs cibles
- Développeurs et contributeurs techniques.
- Utilisateurs avancés recherchant un éditeur minimaliste et rapide.
- Mainteneurs de dépôts souhaitant un outil ergonomique pour la revue.

## Critères de succès
- Démarrage rapide et consommation mémoire raisonnable sur des machines
  modestes (objectif : lancement perceptible < 1 s sur un projet moyen).
- Navigation fluide sur des fichiers volumineux (objectif : 50 000 lignes sans
  latence visible).
- API de plugins stable et documentée.
- Documentation claire pour l’onboarding en moins d’une journée.

## Contraintes
- Priorité à la stabilité du core.
- Documentation en français (anglais si nécessaire pour références).
- Base de code maintenable : modules courts, interfaces explicites.

## Risques principaux
- Complexité du rendu performant multi-plateforme.
- Couplage excessif entre UI et core si les abstractions sont mal définies.
- Sous-estimation de l’effort d’ergonomie (clavier, flux utilisateur).

## Planification macro
- Phase 0 : cadrage, architecture, standards, documentation.
- Phase 1 : MVP avec édition, rendu, navigation, recherche simple.
- Phase 2 : fonctionnalités avancées (highlight, multi-curseurs, LSP-lite).
- Phase 3 : personnalisation (config, thèmes, keymaps, plugins).
- Phase 4 : stabilisation, performance, packaging.

## Livrables attendus par phase
- **Phase 0** : vision, architecture, standards, planification claire.
- **Phase 1** : MVP utilisable (édition, rendu, navigation, recherche simple).
- **Phase 2** : fonctionnalités avancées (highlight, multi-curseurs, LSP-lite).
- **Phase 3** : personnalisation avancée (plugins, thèmes, keymaps, config).
- **Phase 4** : stabilisation, performances, packaging.
