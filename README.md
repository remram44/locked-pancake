# What is this?

This is a stack-based bytecode interpreter for a scripting language I invented. I am taking inspiration from Lua (but aim to make the language more usable) and Python (but want to make it safer to run untrusted code or multiple independent programs).

Goals:

* [ ] Compiler from some high-level grammar to bytecode (not started)
* [ ] Serializable bytecode format
* [x] Bytecode interpreter operating on high-level data types:
    * String, integer, nil
    * Code and function objects (a function object is a closure, e.g. code plus current values of the "upvalues" = closed variables)
    * Class, instance
    * Dictionary, list

Non-goals:

* Extensive standard library
* Extreme speed
* Interoperability with anything
* Taking this too seriously, I'm just messing around
