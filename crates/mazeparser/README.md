# MazeParser

A format for defining mazes and a parser for reading it from text.

## Maze Text Format
| Key                     | Description                                                                                   |
| ----------------------- | --------------------------------------------------------------------------------------------- |
| SP                      | Starting Point. Which cell the mouse starts in. Format: x, y                                  |
| SD                      | Starting Direction. Which direction the mouse should face to start. Can be one of: R, L, U, D |
| FI                      | Finish. Where the finish should be placed. Format: x,y; size                                  |
| FR                      | Maze Friction.                                                                                |
| .R followed by a number | Defines walls in the row with the number after .R                                             |
| .C followed by a number | Defines walls in the column with the number after .C                                          |

Lines without `:` and lines starting with a `#` are ignored.
