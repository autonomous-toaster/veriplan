bool flag = false;
active proctype P() { assert(!flag) }
ltl p0 { [] ( !flag ) }
