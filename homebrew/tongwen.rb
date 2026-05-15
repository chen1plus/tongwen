# Move this file to your tap repo as:
#   homebrew-tongwen/Formula/tongwen.rb
# Then: brew tap <YOUR_GH>/tongwen && brew install tongwen
#
# Until you publish a tagged release, install the head version:
#   brew install --HEAD tongwen
#
# Replace <YOUR_GH> with your GitHub username before publishing.

class Tongwen < Formula
  desc "OpenAI-compatible local Simplified→Traditional Chinese (s2tw) endpoint"
  homepage "https://github.com/<YOUR_GH>/tongwen"
  license "MIT"

  # Update url + sha256 when cutting a tagged release.
  url "https://github.com/<YOUR_GH>/tongwen/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  version "0.1.0"

  head "https://github.com/<YOUR_GH>/tongwen.git", branch: "main"

  depends_on "deno" => :build

  def install
    system "deno", "compile",
           "--allow-net", "--allow-env",
           "--output", "tongwen",
           "src/main.ts"
    bin.install "tongwen"
  end

  service do
    run [opt_bin/"tongwen"]
    keep_alive true
    log_path var/"log/tongwen.log"
    error_log_path var/"log/tongwen.err.log"
    environment_variables TONGWEN_PORT: "1180", TONGWEN_HOST: "127.0.0.1"
  end

  test do
    require "json"
    require "net/http"
    require "timeout"

    port = free_port
    ENV["TONGWEN_PORT"] = port.to_s
    ENV["TONGWEN_HOST"] = "127.0.0.1"

    pid = spawn(bin/"tongwen")
    begin
      Timeout.timeout(10) do
        loop do
          begin
            TCPSocket.new("127.0.0.1", port).close
            break
          rescue Errno::ECONNREFUSED
            sleep 0.1
          end
        end
      end

      uri = URI("http://127.0.0.1:#{port}/v1/chat/completions")
      req = Net::HTTP::Post.new(uri, "Content-Type" => "application/json")
      req.body = { messages: [{ role: "user", content: "汉字" }] }.to_json
      res = Net::HTTP.start(uri.hostname, uri.port) { |h| h.request(req) }
      assert_match "漢字", JSON.parse(res.body).dig("choices", 0, "message", "content")
    ensure
      Process.kill("TERM", pid)
      Process.wait(pid)
    end
  end
end
