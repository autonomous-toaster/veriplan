/* The control: the same model asserting the untouched element. An engine that models an array as one
   variable passes array_element.pml and fails here. */
bool busy[3];

active proctype P() {
	busy[1] = true;
	assert(busy[2])
}
