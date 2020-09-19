# Various Notes On Poorly/Undocumented Cases
This list is a compilation of certain nuances that took me a while to understand, or just plain undocumented cases.

## CPU
* Don't reset the IE register upon executing the DI instruction, this will cause issues. Only disable IME instead.

## MMU

## PPU
* The Window's tile selection is not at all based on the BG/sprites. Instead, it always starts on the top left (0x00) of it's selected tilemap + offset as set by LCD Control.
It keeps track of how many lines it has drawn, and when it has become > 8 it will switch over to the (current tile ypos)+1 line of tiles.
* When WX is 0, ScrollX will start having effects on the window - the gameboy accidentally switches to the window tiles before it performs the fine scroll adjustment - so when WX=0 the window gets scrolled by SCX. Fun!


## Joypad
* Games pretty much never use the Joypad interrupt. 
Instead, they manually poll the Joypad register by setting the select mode.
This means that you do not have to set the select bits yourself! In fact, that will probably break things.
It's therefore easiest to just keep 2 separate lists with the current button inputs and serve the correct one depending on the current mode, as selected by the game.

## Timer
* Blargg's instr_timing test relies on the timer being ticked after
 an interrupt check, which could potentially tick clocks, otherwise it will fail at #255.