// Exporte en C décompilé les fonctions des classes du jeu (pas celles du
// moteur Cocos2d, de Box2D, de la bibliothèque standard, de Lua ou de la
// boutique) : un fichier par classe, dans le dossier donné en argument.
// Un second argument (noms séparés par des virgules) n'exporte que ces
// classes, même celles du jeu au préfixe « CC » (CCTransitionProgressVariant).
//@category NanacaCrash

import java.io.File;
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.data.PointerDataType;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Parameter;
import ghidra.program.model.listing.ParameterImpl;
import ghidra.program.model.listing.Variable;
import ghidra.program.model.symbol.Namespace;
import ghidra.program.model.symbol.SourceType;

public class ExportGame extends GhidraScript {

    /** Les espaces de noms des bibliothèques, laissés de côté. */
    private static final String[] LIBRARIES = {
        "cocos2d", "std", "CocosDenshion", "cocostudio", "extension", "tinyxml2", "rapidjson",
        "CSJson", "Json", "soomla", "__cxxabiv1", "__gnu_cxx", "gui", "ui", "cs", "spine", "kazmath",
    };

    private static boolean library(String name) {
        for (String l : LIBRARIES) {
            if (name.equals(l)) {
                return true;
            }
        }
        return name.startsWith("b2") || name.startsWith("CC") || name.startsWith("Lua")
            || name.startsWith("tolua") || name.startsWith("<");
    }

    /** Le code ARM charge textes et constantes par adresses relatives
     * (« DAT_x + 0xbase » : le mot en x plus le compteur ordinal) : on
     * écrit à côté la chaîne ou la valeur pointée. */
    private static final Pattern RELATIVE = Pattern.compile("DAT_([0-9a-f]{8}) \\+ 0x([0-9a-f]+)");
    /** La même chose quand Ghidra écrit la base comme une adresse :
     * « (int)&DAT_x + DAT_y » vaut le mot en y plus x. */
    private static final Pattern RELATIVE_ADDRESS = Pattern.compile("&DAT_([0-9a-f]{8}) \\+ DAT_([0-9a-f]{8})");
    private static final Pattern CONSTANT = Pattern.compile("(?<![&+] )(?<!&)DAT_([0-9a-f]{8})(?! \\+ 0x)");

    private String annotate(String code) {
        Matcher m = RELATIVE_ADDRESS.matcher(code);
        StringBuilder first = new StringBuilder();
        while (m.find()) {
            String note = "";
            try {
                int word = currentProgram.getMemory().getInt(toAddr(Long.parseLong(m.group(2), 16)));
                long target = (word + Long.parseLong(m.group(1), 16)) & 0xFFFFFFFFL;
                String s = cString(target);
                note = s != null ? " /* \"" + s + "\" */" : String.format(" /* @%08x */", target);
            } catch (Exception e) {
                // Adresse illisible : rien à ajouter.
            }
            m.appendReplacement(first, Matcher.quoteReplacement(m.group() + note));
        }
        m.appendTail(first);
        code = first.toString();
        m = RELATIVE.matcher(code);
        StringBuilder out = new StringBuilder();
        while (m.find()) {
            String note = "";
            try {
                int word = currentProgram.getMemory().getInt(toAddr(Long.parseLong(m.group(1), 16)));
                long target = (word + Long.parseLong(m.group(2), 16)) & 0xFFFFFFFFL;
                String s = cString(target);
                note = s != null ? " /* \"" + s + "\" */" : String.format(" /* @%08x */", target);
            } catch (Exception e) {
                // Adresse illisible : rien à ajouter.
            }
            m.appendReplacement(out, Matcher.quoteReplacement(m.group() + note));
        }
        m.appendTail(out);
        // Les constantes seules : leur valeur en flottant et en entier.
        m = CONSTANT.matcher(out.toString());
        StringBuilder out2 = new StringBuilder();
        while (m.find()) {
            String note = "";
            try {
                long address = Long.parseLong(m.group(1), 16);
                int word = currentProgram.getMemory().getInt(toAddr(address));
                long dword = currentProgram.getMemory().getLong(toAddr(address));
                note = String.format(" /* %s | %d | double %s */", Float.intBitsToFloat(word), word, Double.longBitsToDouble(dword));
            } catch (Exception e) {
                // Adresse illisible : rien à ajouter.
            }
            m.appendReplacement(out2, Matcher.quoteReplacement(m.group() + note));
        }
        m.appendTail(out2);
        return out2.toString();
    }

    /** La chaîne C (UTF-8) à une adresse, si elle en a l'air. */
    private String cString(long address) throws Exception {
        byte[] buf = new byte[256];
        int n = 0;
        while (n < buf.length) {
            byte b = currentProgram.getMemory().getByte(toAddr(address + n));
            if (b == 0) {
                break;
            }
            buf[n++] = b;
        }
        if (n == 0 || n == buf.length) {
            return null;
        }
        String s = new String(buf, 0, n, StandardCharsets.UTF_8);
        for (char c : s.toCharArray()) {
            if (c < 0x20 && c != '\n' && c != '\t' || c == '\uFFFD') {
                return null;
            }
        }
        return s.replace("\\", "\\\\").replace("\n", "\\n").replace("*/", "* /");
    }

    /** Les noms démêlés ne disent pas si une méthode est statique : sans
     * « this » (dans r0), tous les paramètres sont décalés d'un cran. On
     * l'ajoute aux méthodes des classes, sauf aux fabriques et singletons. */
    private void addThis(Function f) throws Exception {
        Namespace parent = f.getParentNamespace();
        String name = f.getName();
        if (parent == null || parent.isGlobal() || f.isThunk() || f.isExternal()
            || !Character.isUpperCase(parent.getName().charAt(0))
            || name.startsWith("create") || name.startsWith("shared") || name.startsWith("purge")
            || name.equals("node") || name.equals("getInstance") || name.equals("shared")) {
            return;
        }
        Parameter[] old = f.getParameters();
        if (old.length > 0 && old[0].getName().equals("this")) {
            return;
        }
        List<Variable> params = new ArrayList<>();
        params.add(new ParameterImpl("this", PointerDataType.dataType, currentProgram));
        for (Parameter p : old) {
            params.add(new ParameterImpl(p.getName(), p.getDataType(), currentProgram));
        }
        f.updateFunction("__stdcall_softfp", null, params,
            Function.FunctionUpdateType.DYNAMIC_STORAGE_ALL_PARAMS, true, SourceType.ANALYSIS);
    }

    @Override
    public void run() throws Exception {
        File out = new File(getScriptArgs()[0]);
        out.mkdirs();
        List<String> only = getScriptArgs().length > 1 ? List.of(getScriptArgs()[1].split(",")) : null;
        // armeabi-v7a passe les flottants dans les registres entiers
        // (« softfp ») : sans cela, Ghidra les cherche dans s0, s1…
        for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
            try {
                f.setCallingConvention("__stdcall_softfp");
                addThis(f);
            } catch (Exception e) {
                // Fonction externe ou figée : laissée telle quelle.
            }
        }
        DecompInterface decompiler = new DecompInterface();
        decompiler.openProgram(currentProgram);
        Map<String, StringBuilder> files = new TreeMap<>();
        for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
            if (monitor.isCancelled()) {
                break;
            }
            // La classe : l'espace de noms juste sous le global.
            Namespace top = f.getParentNamespace();
            if (f.isThunk() || f.isExternal()) {
                continue;
            }
            String name;
            if (top == null || top.isGlobal()) {
                // Une fonction libre n'est exportée que nommée (second argument).
                if (only == null || !only.contains(f.getName())) {
                    continue;
                }
                name = f.getName();
            } else {
                while (top.getParentNamespace() != null && !top.getParentNamespace().isGlobal()) {
                    top = top.getParentNamespace();
                }
                name = top.getName();
            }
            if (only != null ? !only.contains(name) : library(name)) {
                continue;
            }
            DecompileResults r = decompiler.decompileFunction(f, 120, monitor);
            String code = r.decompileCompleted() ? annotate(r.getDecompiledFunction().getC()) : "/* échec : " + r.getErrorMessage() + " */\n";
            files.computeIfAbsent(name, k -> new StringBuilder())
                .append("// ").append(f.getName(true)).append(" @ ").append(f.getEntryPoint()).append("\n")
                .append(code).append("\n");
        }
        for (Map.Entry<String, StringBuilder> e : files.entrySet()) {
            String file = e.getKey().replaceAll("[^A-Za-z0-9_]", "_") + ".c";
            try (PrintWriter w = new PrintWriter(new File(out, file), "UTF-8")) {
                w.print(e.getValue());
            }
        }
        println("Exporté : " + files.size() + " classes dans " + out);
    }
}
