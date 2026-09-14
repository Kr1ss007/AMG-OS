filer — Primary Application for AMGOS
================================================================================
Filer is AMGOS's primary and only always-running application.
It bridges Process 1 (backend) and Process 2 (frontend):
- core/: Process 1 backend (filesystem indexer, Pathfinder search index,
  package inspector, download manager).
- shell/: Process 2 frontend (Pathfinder search bar, file browser panels,
  Zen Browser embed, symmetric installer/uninstaller UI).
- protocol/: Shared contract definitions between core and shell.
================================================================================
