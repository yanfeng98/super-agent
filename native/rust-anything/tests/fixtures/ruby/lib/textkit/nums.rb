module Textkit
  module Nums
    module_function

    # Pages needed for total items.
    # @param total [Integer]
    # @param per_page [Integer]
    # @return [Integer]
    def page_count(total, per_page = 10)
      raise ArgumentError, "per_page must be positive" unless per_page.positive?

      -(-total / per_page)
    end

    # @param values [Array<Float>]
    # @return [Float, nil]
    def mean(values)
      return nil if values.empty?

      values.sum / values.size
    end

    # @param value [Integer]
    # @param low [Integer]
    # @param high [Integer]
    # @return [Integer]
    def clamp(value, low, high)
      value.clamp(low, high)
    end
  end
end
