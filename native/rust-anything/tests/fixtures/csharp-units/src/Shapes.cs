namespace Acme.Geo;

public record Point(double X, double Y);

public class Box
{
    public string Label { get; set; } = "";
    public int[] Sizes { get; set; } = new int[0];
}
