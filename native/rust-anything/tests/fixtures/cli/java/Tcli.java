import java.io.FileDescriptor;
import java.io.FileOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.Locale;

public class Tcli {
    static final PrintStream OUT =
        new PrintStream(new FileOutputStream(FileDescriptor.out), true, StandardCharsets.UTF_8);
    static final PrintStream ERR =
        new PrintStream(new FileOutputStream(FileDescriptor.err), true, StandardCharsets.UTF_8);

    static int usage() {
        ERR.println("usage: tcli upper|len|div|words|copy|now");
        return 1;
    }

    static int run(String[] argv) throws Exception {
        if (argv.length == 0) return usage();
        String cmd = argv[0];
        String[] args = Arrays.copyOfRange(argv, 1, argv.length);
        if (cmd.equals("upper")) {
            OUT.println(String.join(" ", args).toUpperCase(Locale.ROOT));
        } else if (cmd.equals("len") && args.length == 1) {
            OUT.println(args[0].length());
        } else if (cmd.equals("div") && args.length == 2) {
            long a, b;
            try {
                a = Long.parseLong(args[0]);
                b = Long.parseLong(args[1]);
            } catch (NumberFormatException e) {
                ERR.println("error: not a number");
                return 2;
            }
            if (b == 0) {
                ERR.println("error: division by zero");
                return 2;
            }
            OUT.println(a + " / " + b + " = " + (a / b) + " remainder " + (a % b));
        } else if (cmd.equals("words")) {
            String text = new String(System.in.readAllBytes(), StandardCharsets.UTF_8).trim();
            OUT.println(text.isEmpty() ? 0 : text.split("\\s+").length);
        } else if (cmd.equals("copy") && args.length == 2) {
            String text;
            try {
                text = Files.readString(Path.of(args[0]));
            } catch (Exception e) {
                ERR.println("error: cannot read " + args[0]);
                return 1;
            }
            try {
                Files.writeString(Path.of(args[1]), text.toUpperCase(Locale.ROOT));
            } catch (Exception e) {
                ERR.println("error: cannot write " + args[1]);
                return 1;
            }
            OUT.println("copied " + text.length() + " characters");
        } else if (cmd.equals("now")) {
            OUT.println("now: " + System.nanoTime());
        } else {
            return usage();
        }
        return 0;
    }

    public static void main(String[] argv) throws Exception {
        System.exit(run(argv));
    }
}
