using System;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.RegularExpressions;

static class Tcli
{
    static int Usage()
    {
        Console.Error.WriteLine("usage: tcli upper|len|div|words|copy|now");
        return 1;
    }

    static int Count(string text) => text.EnumerateRunes().Count();

    static int Run(string[] argv)
    {
        if (argv.Length == 0)
        {
            return Usage();
        }
        var cmd = argv[0];
        var args = argv[1..];
        if (cmd == "upper")
        {
            Console.WriteLine(string.Join(" ", args).ToUpperInvariant());
        }
        else if (cmd == "len" && args.Length == 1)
        {
            Console.WriteLine(Count(args[0]));
        }
        else if (cmd == "div" && args.Length == 2)
        {
            var number = NumberStyles.AllowLeadingSign;
            if (!long.TryParse(args[0], number, CultureInfo.InvariantCulture, out var a)
                || !long.TryParse(args[1], number, CultureInfo.InvariantCulture, out var b))
            {
                Console.Error.WriteLine("error: not a number");
                return 2;
            }
            if (b == 0)
            {
                Console.Error.WriteLine("error: division by zero");
                return 2;
            }
            Console.WriteLine($"{a} / {b} = {a / b} remainder {a % b}");
        }
        else if (cmd == "words")
        {
            var text = Console.In.ReadToEnd().Trim();
            Console.WriteLine(text.Length == 0 ? 0 : Regex.Split(text, @"\s+").Length);
        }
        else if (cmd == "copy" && args.Length == 2)
        {
            string text;
            try
            {
                text = File.ReadAllText(args[0]);
            }
            catch (Exception)
            {
                Console.Error.WriteLine($"error: cannot read {args[0]}");
                return 1;
            }
            try
            {
                File.WriteAllText(args[1], text.ToUpperInvariant());
            }
            catch (Exception)
            {
                Console.Error.WriteLine($"error: cannot write {args[1]}");
                return 1;
            }
            Console.WriteLine($"copied {Count(text)} characters");
        }
        else if (cmd == "now")
        {
            Console.WriteLine($"now: {Stopwatch.GetTimestamp()}");
        }
        else
        {
            return Usage();
        }
        return 0;
    }

    static int Main(string[] argv)
    {
        Console.OutputEncoding = new UTF8Encoding(false);
        Console.InputEncoding = new UTF8Encoding(false);
        Console.Out.NewLine = "\n";
        Console.Error.NewLine = "\n";
        return Run(argv);
    }
}
