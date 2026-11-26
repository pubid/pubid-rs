#!/usr/bin/env ruby
# frozen_string_literal: true

# The Ruby engine of the differential runner. Cases go in as one JSON
# file (--cases path), the verdicts come back as one JSON object per
# line: an engine header first (availability + version), then one
# verdict per case in input order. The pubid gem loads either from the
# gem installation or from PUBID_RUBY_LIB (a checkout's lib/).

require "json"

case_index = ARGV.index("--cases")
cases_path = case_index && ARGV[case_index + 1]
unless cases_path
  puts JSON.generate({ "status" => "engine", "available" => false,
                       "detail" => "no --cases path" })
  exit 0
end

lib = ENV["PUBID_RUBY_LIB"]
$LOAD_PATH.unshift(lib) if lib && !lib.empty? && Dir.exist?(lib)

begin
  require "pubid"
  Pubid.eager_load_flavors!
rescue StandardError => e
  puts JSON.generate({ "status" => "engine", "available" => false,
                       "detail" => "#{e.class}: #{e.message}" })
  exit 0
end

version = defined?(Pubid::VERSION) ? Pubid::VERSION.to_s : "unknown"
puts JSON.generate({ "status" => "engine", "available" => true, "version" => version })

cases = JSON.parse(File.read(cases_path))
cases.each do |entry|
  flavor_module = Pubid::Registry.get(entry["flavor"])
  if flavor_module.nil?
    puts JSON.generate({ "i" => entry["i"], "status" => "unimplemented" })
    next
  end
  begin
    id = flavor_module.parse(entry["input"])
    hash = Pubid::Conformance.plainify(id.to_hash)
    urn = id.respond_to?(:to_urn) ? id.to_urn : nil
    puts JSON.generate({ "i" => entry["i"], "status" => "ok", "hash" => hash,
                         "human" => id.to_s, "urn" => urn })
  rescue StandardError => e
    code = defined?(Pubid::Conformance) ? Pubid::Conformance.error_code_for(e) : e.class.name
    puts JSON.generate({ "i" => entry["i"], "status" => "reject", "error_code" => code })
  rescue Exception => e # rubocop:disable Lint/RescueException -- a crash is a verdict, not a kill
    puts JSON.generate({ "i" => entry["i"], "status" => "error", "detail" => e.class.name })
  end
end
