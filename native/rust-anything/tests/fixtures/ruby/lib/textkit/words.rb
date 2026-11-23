module Textkit
  # Word helpers.
  module Words
    # Lowercases text and joins its words with sep.
    # @param text [String]
    # @param sep [String]
    # @param max [Integer, nil] cut the slug to this many characters
    # @return [String]
    def self.slugify(text, sep = "-", max: nil)
      slug = text.downcase.scan(/[[:alnum:]]+/).join(sep)
      max ? slug[0, max].chomp(sep) : slug
    end

    # @param text [String]
    # @return [Hash{String => Integer}]
    def self.word_count(text)
      text.split.map(&:downcase).tally
    end

    # @param text [String]
    # @return [String]
    def self.titleize(text)
      text.split(/(\s+)/).map(&:capitalize).join
    end

    def self.each_word(text)
      text.split.each { |w| yield w }
    end
  end
end
