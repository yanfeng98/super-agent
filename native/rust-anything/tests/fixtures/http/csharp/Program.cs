using System.Diagnostics;
using System.Globalization;
using System.Text;
using System.Text.RegularExpressions;

const string Text = "text/plain; charset=utf-8";

var builder = WebApplication.CreateBuilder(args);
builder.Logging.ClearProviders();
var app = builder.Build();

static async Task Reply(HttpContext ctx, int status, string type, string body)
{
    ctx.Response.StatusCode = status;
    ctx.Response.ContentType = type;
    await ctx.Response.WriteAsync(body);
}

app.Run(async ctx =>
{
    var method = ctx.Request.Method;
    var path = ctx.Request.Path.Value ?? "/";
    var parts = path.Split('/').Skip(1).ToArray();
    if (method == "GET" && path == "/hello")
    {
        await Reply(ctx, 200, "application/json", "{\"message\":\"hello\"}");
    }
    else if (method == "POST" && path == "/upper")
    {
        using var buffer = new MemoryStream();
        await ctx.Request.Body.CopyToAsync(buffer);
        var body = Encoding.UTF8.GetString(buffer.ToArray());
        await Reply(ctx, 200, Text, body.ToUpperInvariant());
    }
    else if (method == "GET" && parts.Length == 3 && parts[0] == "div")
    {
        var number = new Regex(@"\A-?\d+\z");
        var (a, b) = (parts[1], parts[2]);
        if (!number.IsMatch(a) || !number.IsMatch(b)
            || !long.TryParse(a, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var x)
            || !long.TryParse(b, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var y))
        {
            await Reply(ctx, 400, Text, "error: not a number\n");
        }
        else if (y == 0)
        {
            await Reply(ctx, 400, Text, "error: division by zero\n");
        }
        else
        {
            await Reply(ctx, 200, Text, $"{x} / {y} = {x / y} remainder {x % y}\n");
        }
    }
    else if (method == "GET" && path == "/now")
    {
        await Reply(ctx, 200, Text, $"now: {Stopwatch.GetTimestamp()}\n");
    }
    else
    {
        await Reply(ctx, 404, Text, "not found\n");
    }
});

app.Run();
