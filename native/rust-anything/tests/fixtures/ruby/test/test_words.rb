require "minitest/autorun"
require_relative "../lib/textkit"

class WordsTest < Minitest::Test
  def test_slugify
    assert_equal "hello-world", Textkit::Words.slugify("Hello, World!")
  end
end
