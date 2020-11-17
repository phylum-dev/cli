#!/usr/bin/env ruby
require 'bundler'

Package = Struct.new(:name, :version)


def usage
  puts "usage: #{$0} <Gemfile.lock>"
  exit
end

def call_cli(package_list)
  pkg_info = package_list.map { |p| "#{p.name}:#{p.version}\n" }.join
  IO::popen('phylum-cli batch -t ruby', 'w') do |f|
    f.puts pkg_info
  end
  exit $?.exitstatus
end

if ARGV.length == 0
  lockfile = Bundler.default_lockfile
elsif ARGV.length == 1
  lockfile = ARGV[0]
else
  usage
end


begin
  p = Bundler::LockfileParser.new(Bundler.read_file(lockfile))
  pkgs = p.specs.map { |spec| Package.new(spec.name, spec.version.to_s) }
rescue => e
  puts "error: #{e.message}"
  usage
end

if pkgs.length == 0
  puts "No valid packages found in #{lockfile}"
  usage
end

puts "Submitting request for #{pkgs.length} packages =>"
call_cli(pkgs)
