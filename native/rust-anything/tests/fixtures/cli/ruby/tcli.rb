def usage
  warn "usage: tcli upper|len|div|words|copy|now"
  1
end

def run(argv)
  return usage if argv.empty?

  cmd, *args = argv.map { |a| a.dup.force_encoding("UTF-8") }
  if cmd == "upper"
    puts args.join(" ").upcase
  elsif cmd == "len" && args.size == 1
    puts args[0].length
  elsif cmd == "div" && args.size == 2
    unless args.all? { |a| a.b.match?(/\A-?\d+\z/) }
      warn "error: not a number"
      return 2
    end
    a, b = args.map(&:to_i)
    if b.zero?
      warn "error: division by zero"
      return 2
    end
    puts "#{a} / #{b} = #{a / b} remainder #{a % b}"
  elsif cmd == "words"
    puts $stdin.read.b.split.size
  elsif cmd == "copy" && args.size == 2
    begin
      text = File.read(args[0], encoding: "UTF-8")
    rescue SystemCallError
      warn "error: cannot read #{args[0]}"
      return 1
    end
    begin
      File.write(args[1], text.upcase)
    rescue SystemCallError
      warn "error: cannot write #{args[1]}"
      return 1
    end
    puts "copied #{text.length} characters"
  elsif cmd == "now"
    puts "now: #{Process.clock_gettime(Process::CLOCK_REALTIME, :nanosecond)}"
  else
    return usage
  end
  0
end

exit run(ARGV)
