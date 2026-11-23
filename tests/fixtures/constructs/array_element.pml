/* A global array with per-element storage: writing index 1 must not disturb index 2. */
bool busy[3];

active proctype P() {
	busy[1] = true;
	assert(busy[1])
}
