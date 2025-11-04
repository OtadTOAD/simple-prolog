# References

https://www.youtube.com/watch?v=QGXypIkV-GU
https://raw.githubusercontent.com/unimorph/eng/master/eng

Maybe in future we can make one that supports Georgian.
(https://github.com/bumbeishvili/GeoWordsDatabase)

# Goals

- Natural language -> Prolog translator.
    - Break down natural text chunk, into simple sentences.
    - Match 'patterns' on this simple sentence using database.
    - If pattern is not found, log the unknown sentence for further work.
- Ability to manage rules and interact with database.
    - Ability to create word entry.
    - Ability to create sentence patterns.
    - Ability to browse database.
- Ability to run queries on the prolog output.
    - Check truth of statements.
    - Find unknowns.