bool a = true;
bool b = false;
active proctype P() { b = a }
ltl p0 { [] ( b || !b ) }
