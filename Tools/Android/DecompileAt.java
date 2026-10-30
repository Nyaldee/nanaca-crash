// Décompile en C les fonctions aux adresses données (hexadécimal, adresses
// Ghidra), y compris celles du moteur que ExportGame laisse de côté.
//@category NanacaCrash

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;

public class DecompileAt extends GhidraScript {
    @Override
    public void run() throws Exception {
        DecompInterface decomp = new DecompInterface();
        decomp.openProgram(currentProgram);
        for (String arg : getScriptArgs()) {
            // Adresse hexadécimale, ou à défaut nom de fonction (Classe::methode)
            Function f = arg.matches("[0-9a-fA-F]+")
                ? getFunctionContaining(toAddr(Long.parseLong(arg, 16)))
                : findByName(arg);
            if (f == null) {
                println("// rien à " + arg);
                continue;
            }
            DecompileResults r = decomp.decompileFunction(f, 60, monitor);
            println("// " + f.getName(true) + " @ " + f.getEntryPoint());
            println(r.getDecompiledFunction() == null ? "// échec" : r.getDecompiledFunction().getC());
        }
        decomp.dispose();
    }

    private Function findByName(String name) {
        for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
            if (f.getName(true).endsWith(name)) {
                return f;
            }
        }
        return null;
    }
}
