# README

**Name:**	Kyle Lin

**Period:**	4

**Game Title:** Snipe Hunt

## Running

Run the `main` method of `SnipeHunt`.

To learn how to play, **complete the interactive in-game tutorial**.
Alternatively, you can read the [user guide](https://docs.google.com/document/d/1fTrR5vQknKrvPBtWhrVsf_ePw8wuyltaq6RTW-e76SI/edit?usp=sharing.), but this is much less fun in my opinion.

## Game Proposal

I plan on building a digital card game inspired by chess, which will have cards instead of pieces.
There will be two players, and each player will have a snipe (card) and some supporting animals (also cards).
Each card will be on a row, and users will be able to move cards up or down a row each turn as dictated by the rules.
The rules will also specify how each card can capture other cards.
The objective will be to capture your opponent's snipe (analogous to the king).
Unlike chess, the cards will be shuffled before being distributed, so players will have a different assortment of cards each game.
The exception is that snipe distribution will be fixed (i.e., each player will have exactly one snipe every game).

I plan on making this a two-player pass-and-play game (i.e., two people will sit in front of the same computer to play this game).
If I have time, I can implement a single player mode with an AI component, but this seems extremely complicated and I highly doubt I will get this far in the allotted time.

Game Controls:

- Mouse

Game Elements:

- "Animals" (cards) which can be moved by each user.

How to Win:

- Capture the opponent's snipe.

## Link Examples
As this is an original game idea, I can't give a link to an existing implementation (because there is none).

The next best thing would be to complete the in-game interactive tutorial.
Alternatively, the you can read the [user guide](https://docs.google.com/document/d/1fTrR5vQknKrvPBtWhrVsf_ePw8wuyltaq6RTW-e76SI/edit?usp=sharing.), but as previously mentioned, I think the latter is much less fun.

## Teacher Response

**Approved**

The key with this kind of game will be making sure gameplay is balanced.  For example, if the opening random distrubution leads to highly imbalanced hands then whichever player has the better hand will win easily.  But as long as each game relies mostly on how players cleverly use their cards, this will be a fun and engaging game worth continuing and publishing :)  One idea for starting the game is to give both players the same random hand and then give them a certain number of modifications they can make to that starting hand.  For example, maybe players get to move three cards or maybe they can swap out 1 card for another.  Or maybe the players get 10 gems to spend to modify their starting hand.

You don't have to use the World / Actor system we used for the GameEngine project and I don't think it would be useful here anyway.  What would be useful (after card mechanics are working) is to throw in some animation using transition effects or frame animation.  But focus on mechanics first.

Finally, when planning the game, use paper cards and physically play against yourself or your parents using different configurations to see what works and what doesn't.

## Class Design and Brainstorm

I initially tried hard-coding the interactive tutorial, but the tutorial was really long and it became a huge hassle to edit.
I ultimately decided to instead write the tutorial in Markdown, because that was much easier to work with.
I then proceeded to create a rudimentary parser that rendered the Markdown as JavaFX nodes.
The parser took care of the event handling and styling for me, leaving me free to focus exclusively on the content of the tutorial.

## Development Journal

Every day you work, keep track of it here.

### 14 May 2020 (20 minutes)

**Goal:** 

Write basic proposal

**Work accomplished:**

I wrote a basic proposal, but I still need to write the complete rulebook.

### 15 May 2020 (90 minutes)

**Goal:**

Develop game rules.

**Work accomplished:**

I just played against myself for an hour and a half.

It's really hard for either side to win. I can think of two possible explanations:

1. There **exists a good attacking strategy**, but I **haven't discovered it yet**. - Think about how it feels when you're learning chess. In the beginning, you don't even know where to start. It takes many games before you finally begin to recognize certain patterns and develop decent strategy. Even then, the progress made was probably only made possible by observing an expert (be it in the form of playing someone good (or a computer), or reading chess books). If you are limited to playing yourself (or someone else who has also never played before), your progress will be significantly slowed. Of course, since this game is new, there aren't any experts I can learn from.
2. There is simply insufficient material to force a checkmate. In other words, it's not that I'm bad at the game; it's the game that's broken. In this case, I will either have to make the cards more powerful or add more cards. 

It's probably a mix of both.

Also, the random card distribution can lead to extremely skewed material imbalances, so I will need to revise the rules accordingly.

### 16 May 2020 (8 hours)

**Goal:**

Implement a digital version and create a computer to play against so I can accelerate how fast I learn good strategy.

**Work accomplished:**

I created a simple proof-of-concept on the Web (not Java).
There are several bugs.
I still have yet to create the AI.

### 17 May 2020 (4 hours)

**Goal:**

Fix the bugs.

**Work accomplished:**

I fixed most of the bugs, but I still need to make the engine detect if a player is out of legal moves so it can declare that player the loser.


### 18 May 2020 (4 hours)

**Goal:**

Start working on the AI.

**Work accomplished:**

I implemented a basic minimax algorithm, but due to exponential growth, it takes forever to compute to a depth of 3, and just hangs if you attempt a depth of 4.

Also, minimax requires an evaluator function, but I don't trust my estimates for how many points each card is worth, seeing as I haven't played that many games.

The algorithm also fails when a player runs out of legal moves, so I'll have to fix that later.


### 19 May 2020 (2 hours)

**Goal:**

Fix AI bugs.

**Work accomplished:**

I fixed some of the bugs, but the computer's performance is so horrible I decided to give up on the current algorithm.


### 20-22 May 2020 (2 hours)

**Goal:**

Find a better algorithm.

**Work accomplished:**

I tried looking into AlphaZero but I don't know anything about neural networks and it seems really complicated. I read several explanations, and they all refer to something called Monte Carlo Tree Search (MCTS), but I still don't understand how that works.


### 23 May 2020 (4 hours)

**Goal:**

Redesign game rules and continue researching game-playing algorithms.

**Work accomplished:**

I initially wanted to implement an AI first, which I would use to find the deficiencies in the game mechanics. At this point, with the project due date a week away, I realize that that is too ambitious, so I decided to redesign the rules relying only on human intuition.

Notable changes:

1. I decided to double the amount of cards, so there will be sufficient material to launch a decent attack.

2. I also decided to shuffle and distribute the major cards and minor cards separately, so each player will receive the equal amounts of each. Previously, it was possible for a player to be dealt all the major cards, and several minor cards, and the opponent to be dealt all minor cards.

3. Remove promotion and demotion. Before, each card except the Snipe had two forms: unpromoted and promoted. That made it take way longer to analyze positions, so I removed that feature.

I played against myself twice, and then against my parents thrice.
It seems like it has potential.
Of course, we can't know for certain until our strategic understanding of the game matures.


### 23 May 2020 (20 minutes)

**Goal:**

Continue researching game-playing algorithms.

**Work accomplished:**

I think I finally get the basic idea behind MCTS.

### 24-25 May 2020 (10 hours)

**Goal:**

Implement second draft of the web implementation (with updated rules) and MCTS.

**Work accomplished:**

I still haven't gotten the game finished, but I got a pretty good start.

If you're wondering why I'm creating a TypeScript (basically JavaScript) draft before writing the Java final draft, it's because I'm far more comfortable with the former, so I would prefer to use it for ungraded experimentation (i.e., exploring MCTS). If I don't finish by Friday, I will abandon the web implementation and begin working on the Java implementation in order to ensure that I can finish by the due date. However, if this is the case, the game would only support two-player pass-and-play (i.e., there would not be a computer to play against).

### 26-29 May 2020 (20 hours)

**Goal:**

Implement second draft of the web implementation and MCTS.

**Work accomplished:**

The game is functional. The user controls both sides.

The AI only half-works. It's not too bad at winning but it's terrible at avoiding losing. I'm pretty sure there's a bug in my implementation of MCTS, because at least to my understanding, the algorithm should converge on optimal play (even in situations where the best move is defensive) with enough simulations.

### 30 May 2020 to 1 June 2020 (18 hours)

**Goal:**

Implement the game in Java:

- Design digital card graphics
- Code the game logic
- Write a user guide

**Work accomplished:**

The game itself is functional.

There also is a non-interactive textual user guide that the user can read to learn the game.

I personally prefer interactive tutorials to lengthy rulebooks, so I started implementing one, but ran out of time.


### 1-3 June 2020 (15 hours)

**Goal:**

Create an interactive tutorial.

**Work accomplished:**

I finished building the interactive tutorial.

I ended up writing the tutorial in Markdown (in [./src/resources/tutorial.md](./src/resources/tutorial.md)) and then creating a parser that translated the file into JavaFX Nodes.
