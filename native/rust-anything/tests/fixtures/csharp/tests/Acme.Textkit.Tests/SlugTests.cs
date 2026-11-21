using Acme.Textkit.Text;
using Xunit;

namespace Acme.Textkit.Tests;

public class SlugTests
{
    [Fact]
    public void FoldsAccents() => Assert.Equal("creme-brulee", Slug.Slugify("Crème brûlée"));
}
