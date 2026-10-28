// Affiche les instructions d'une plage d'adresses (arguments : début et
// fin en hexadécimal, adresses Ghidra), pour lire ce que le décompilateur
// a perdu.
//@category NanacaCrash

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Instruction;

public class Disasm extends GhidraScript {
    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        Address start = toAddr(Long.parseLong(args[0], 16));
        Address end = toAddr(Long.parseLong(args[1], 16));
        Instruction i = getInstructionAt(start);
        if (i == null) {
            i = getInstructionAfter(start);
        }
        while (i != null && i.getAddress().compareTo(end) <= 0) {
            println(i.getAddress() + "  " + i);
            i = i.getNext();
        }
    }
}
