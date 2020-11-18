# Various Notes On Poorly/Undocumented Cases
This list is a compilation of certain nuances that took me a while to understand, or just plain undocumented cases.

## CPU
* Don't reset the IE register upon executing the DI instruction, this will cause issues. Only disable IME instead.
* Apparently we fetch an opcode THEN check for interrupts, and if so discard the opcode (and decrement PC) and launch into 
an interrupt routine. This would have effects on timing. MoonEye's ie_push.gb is a good test for this apparently.
* Ensure you run the Blargg instr_timing.gb test WITH bootrom, it seems to rely on some particular state in order to pass.

## MMU

## PPU
* The Window's tile selection is not at all based on the BG/sprites. Instead, it always starts on the top left (0x00) of its selected tilemap + offset as set by LCD Control.
It keeps track of how many lines it has drawn, and when it has become > 8 it will switch over to the (current tile ypos)+1 line of tiles.
* When WX is 0, ScrollX will start having effects on the window - the Gameboy accidentally switches to the window tiles before it performs the fine scroll adjustment - so when WX=0 the window gets scrolled by SCX. Fun!
* The window does not actually check every scanline if WY >= LY, instead it always checks (even if the window is disabled!)
whether LY == WY. As soon as that has happened *once* in the frame, the window will be drawn for the entire remainder of said frame.
This can then only be disabled by toggling the window enable bit in LCD Control.
Apparently Pokemon Crystal relies on this behaviour, a good test for this behaviour can be found [here](https://github.com/Powerlated/TurtleTests/releases/tag/v1.0)
* For some obscure PPU bugs for the DMG [this](http://www.devrs.com/gb/files/faqs.html#GBBugs) is a pretty good resource.
* For behaviour when it comes to sprite priority and 0xFF4C and 0xFF6C refer to: [mattcurie's video](https://www.youtube.com/watch?v=ZaXHkUwLh5U)

## Joypad
* Games pretty much never use the Joypad interrupt. 
Instead, they manually poll the Joypad register by setting the select mode.
This means that you do not have to set the select bits yourself! In fact, that will probably break things.
It's therefore easiest to just keep 2 separate lists with the current button inputs and serve the correct one depending on the current mode, as selected by the game.

## Tests
* We don't pass `call_timing2`, `push_timing`, `rst_timing`, `call_cc_timing2` due to 
instant OAM transfer, would need to switch to gradual transfer to pass these, doesn't seem worth it.
* In `oam_dma_start` at the end of test_round_1 we're supposed to execute one INC B, where B
would then be set to 0x01. We however immediately execute RST which should occur 1 cycle later? (keep in mind the actual PC is PC-1) Log example:
```
[TRACE] (1) Executing opcode: 0021 registers: PC:0197 SP:fffe A:80 F:11000000 B:00 C:00 D:04 E:d8 H:ff L:40 - IE: 00 - IF: 01 - ime: false - name: load_16bit HL DIRECT  
[TRACE] (1) Executing opcode: 00C3 registers: PC:019a SP:fffe A:80 F:11000000 B:00 C:00 D:04 E:d8 H:ff L:46 - IE: 00 - IF: 01 - ime: false - name: jump Always           
[TRACE] (1) Executing opcode: 0077 registers: PC:fe00 SP:fffe A:80 F:11000000 B:00 C:00 D:04 E:d8 H:ff L:46 - IE: 00 - IF: 01 - ime: false - name: load_8bit InstructionAddress(HLI) Reg8(A)
[INFO] Attempted read of blocked OAM, transfer ongoing: true
[TRACE] (1) Executing opcode: 00FF registers: PC:fe01 SP:fffe A:80 F:11000000 B:00 C:00 D:04 E:d8 H:ff L:46 - IE: 00 - IF: 01 - ime: false - name: rst 56                
[TRACE] (1) Executing opcode: 003E registers: PC:0039 SP:fffc A:80 F:11000000 B:00 C:00 D:04 E:d8 H:ff L:46 - IE: 00 - IF: 01 - ime: false - name: load_8bit A DIRECT
```