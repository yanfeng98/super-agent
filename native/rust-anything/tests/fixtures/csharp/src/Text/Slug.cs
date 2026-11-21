using System;
using System.Text.RegularExpressions;
using Acme.Textkit.Util;

namespace Acme.Textkit.Text;

public static class Slug
{
    public static string Slugify(string text, string sep = "-")
    {
        var ascii = Ascii.Fold(text);
        var slug = Regex.Replace(ascii.ToLowerInvariant(), "[^a-z0-9]+", sep);
        return slug.Trim(sep.ToCharArray());
    }

    public static string Truncate(string text, int max)
    {
        if (max < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(max), "max must not be negative");
        }
        return text.Length <= max ? text : text.Substring(0, max);
    }
}
