1. Show more than one ply in the suggestion line of play
2. Validate that each player has 4 Major Animals in `InitialStateBuilder::build`
   - We can also add a `InitialStateBuilder::build_without_major_balance_check`, to allow API users to deliberately give one player a Major Animal advantage.
3. Replace "Open field" text with "Empty rank" text
4. Move the leading animal step UI to the top. It should say, for example, "Ply 65β. Elephant 3, ..."
