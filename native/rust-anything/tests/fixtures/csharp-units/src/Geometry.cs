using System;
using System.Collections.Generic;
using System.Linq;

namespace Acme.Geo;

public static class Geometry
{
    public static double Distance(Point a, Point b)
    {
        var dx = a.X - b.X;
        var dy = a.Y - b.Y;
        return Math.Sqrt(dx * dx + dy * dy);
    }

    public static Point Midpoint(Point a, Point b) => new Point((a.X + b.X) / 2, (a.Y + b.Y) / 2);

    public static int Total(Box parcel)
    {
        var total = 0;
        foreach (var s in parcel.Sizes)
        {
            total = checked(total + s);
        }
        return total;
    }

    public static double Mean(double[] xs)
    {
        if (xs.Length == 0)
        {
            throw new InvalidOperationException("mean of nothing");
        }
        var sum = 0.0;
        foreach (var x in xs)
        {
            sum += x;
        }
        return sum / xs.Length;
    }

    public static int Clamp(int value, int lo = 0, int hi = 100)
    {
        if (lo > hi)
        {
            throw new ArgumentException("lo is above hi");
        }
        return Math.Max(lo, Math.Min(hi, value));
    }

    public static List<string> Words(string text) =>
        text.Split(' ', StringSplitOptions.RemoveEmptyEntries).ToList();

    public static Dictionary<string, int> CountWords(string text)
    {
        var counts = new Dictionary<string, int>();
        foreach (var w in text.Split(' ', StringSplitOptions.RemoveEmptyEntries))
        {
            counts[w] = counts.TryGetValue(w, out var n) ? n + 1 : 1;
        }
        return counts;
    }

    public static int? FirstIndex(string text, string needle)
    {
        var i = text.IndexOf(needle, StringComparison.Ordinal);
        return i < 0 ? null : i;
    }

    public static T Identity<T>(T value) => value;

    public static decimal Price(decimal amount) => amount * 1.2m;

    public static bool TryHalf(int value, out int half)
    {
        half = value / 2;
        return value % 2 == 0;
    }
}
