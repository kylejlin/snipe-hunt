# Mission 7: Write a Snipe Hunt AI in Rust

As mentioned in the readme, the TypeScript implementation already has a Snipe Hunt AI, but it's very slow and very weak.
I want you to build something stronger.

This is intentionally an open-ended mission.
You have complete freedom over the architecture and algorithms you use--minimax, empirical heuristics, MCTS, neural networks--_anything_ is fair game.

The ultimate goal is to create an AI that can crush a human _quickly_ (similar to how Stockfish with 5 seconds per move can crush 99.9% of humans, even if the human has unlimited thinking time).

Some requirements:

- The UI is a webapp
- The UI is user-friendly (e.g., like the TypeScript webapp)
- The UI shows not only the board, but also the ply history (like the TypeScript webapp)
- The user can go Back and Forward in the ply history (like the TypeScript webapp)
- The game state is persistent (e.g., `localStorage`), and can be reset by the user at will
- The user can turn AI analysis ON and OFF at will. There is a dropdown with 3 options:
  - "Computer plays as Alpha" - The AI analysis is ON when it's Alpha's turn, and OFF when it's Beta's turn.
  - "Computer plays as Beta" - Opposite of the above.
  - "Manual" - The user manually turns the AI analysis ON and OFF with a checkbox. This allows the analysis to be toggled independently of whose turn it is.
  - As a consequence of having these 3 options, the web app can support all the human/computer combinations: player-vs-computer, player-vs-player (pass-and-play), and computer-vs-computer.
- The user can set a time limit for the AI analysis. The default is 5 seconds.

Some suggestions (these are not required, though):

- Either use clientside WASM (preferable) for the AI, or do it natively on the serverside (not ideal, but still acceptable). TypeScript is too slow.
- Make the UI look more modern, stylistically
