amgos-desktop
================================================================================
Process 2 — Desktop Session (Rust)

Owns:
- Wayland compositor (forked cosmic-comp, Smithay-based)
- All rendered UI surfaces
- Animation and Interface Engine (cubic-bezier timing system)
- Top Global Menu Bar (Space Orange rectangle identity mark, full width)
- Smart Dock (auto-hide / auto-show)
- Pilot Control (virtual desktop organizer and app thumbnail switcher)
- Window chrome traffic lights (Space Orange close, Space White minimize, Sky Blue maximize)
- Setup wizard (first boot onboarding)
- Lockscreen
- Shutdown ("goodbye") and restart ("i'll see you in a bit") screens
- EventBus (e-bus) subscriber client

Boundary Rule:
Process 2 never touches hardware directly.
Process 2 never holds network credentials.
Process 2 never writes to the filesystem directly.
================================================================================
