using System.Globalization;
using System.Linq;
using System.Text;
using Newtonsoft.Json;

namespace Acme.Textkit.Util;

public static class Ascii
{
    public static string Fold(string text)
    {
        var decomposed = text.Normalize(NormalizationForm.FormD);
        return new string(decomposed
            .Where(c => CharUnicodeInfo.GetUnicodeCategory(c) != UnicodeCategory.NonSpacingMark)
            .ToArray());
    }

    public static string Quote(string text) => JsonConvert.SerializeObject(text);
}
