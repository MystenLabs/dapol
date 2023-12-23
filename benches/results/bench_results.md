# Process and results for benchmarks

## Commands for setting up new EC2 instance for bench marking

```bash
sudo apt update && sudo apt upgrade -y
sudo reboot now
sudo apt install -y build-essential gcc pkg-config libssl-dev && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs >> ./rustup-init.sh && chmod +x ./rustup-init.sh && ./rustup-init.sh -y --no-modify-path && source "$HOME/.cargo/env"

# https://stackoverflow.com/questions/42317062/how-to-monitor-ec2-instances-by-memory
sudo wget https://amazoncloudwatch-agent.s3.amazonaws.com/ubuntu/amd64/latest/amazon-cloudwatch-agent.deb && sudo dpkg -i -E ./amazon-cloudwatch-agent.deb && echo '{"metrics":{"metrics_collected":{"mem":{"measurement":["mem_used_percent"],"metrics_collection_interval":30}}}}' | sudo tee -a /opt/aws/amazon-cloudwatch-agent/bin/config.json > /dev/null && sudo /opt/aws/amazon-cloudwatch-agent/bin/amazon-cloudwatch-agent-ctl -a fetch-config -m ec2 -c file:/opt/aws/amazon-cloudwatch-agent/bin/config.json -s

git clone https://github.com/silversixpence-crypto/dapol

# https://unix.stackexchange.com/questions/99334/how-to-fill-90-of-the-free-memory
cat <(head -c 500m /dev/zero) <(sleep 300) | tail

# increase tmux history length
tmux set-option -g history-limit 50000 \; new-session

MAX_ENTITIES=2000000 cargo bench --bench manual_benches
``**

## Summary of files

**original_bench_data.csv**: First run of the benchmarks after they were written. This was run on a laptop (Macbook pro but specs unknown**.

**r7a.4xlarge_memory.csv**: Currently the only readings for small input sizes.

**r7a.32xlarge_memory.csv**: Currently the only readings for large input sizes.

## Timeline notes

With all these readings the tree was using a full node (liability, blinding factor, hash, Pedersen commitment)

<span class="timestamp-wrapper"><span class="timestamp">[2023-12-13 Wed] </span></span> tried to run all benches on c7a.16xlarge

-   The plan was to run up to 1M entities on criterion<sub>benches.rs</sub>, then do the rest on the manual<sub>benches.rs</sub>
-   criterion<sub>benches</sub>
    -   number of entities: up to 1M
    -   heights: 16, 32, 64
    -   max thread counts: 16, 32, 48, 64
    -   I mistakenly deleted all the stdout, so the only data that survived was the Criterion timing, not the mem usage or tree serialization data
-   manual<sub>benches</sub>
    -   Only run for: height 32, max<sub>thread</sub><sub>count</sub> 16, num<sub>entities</sub> 1<sub>000</sub><sub>000</sub> - 10<sub>000</sub><sub>000</sub>

Then we got an r7a.4xlarge

-   the store depth was changed so that all the nodes were stored, which gives the greatest mem & tree file readings
-   had problems with running out of memory, and storage space
-   also the data was not very accurate because only 1 run of each was done
-   we also lost a whole file of data 'cause saving stdout to a file gave an empty file
-   height [16,32], max<sub>thread</sub><sub>count</sub> [4,8,12,16], num<sub>entities</sub> 10<sub>000</sub>-10<sub>000</sub><sub>000</sub>
-   for height 64 we only got to max<sub>thread</sub><sub>count</sub>=4 and num<sub>entities</sub>=3<sub>000</sub><sub>000</sub> 'cause we ran out of memory

At this point I tried pre-allocating capacity for the DashMap so that it didn't have these massive memory jumps which would use more than necessary.

Got an r7a.32xlarge

-   realized there is a bug where the thread pool is not fully utilized, which means all the build time benches up to this point are slower than post-bug fix
-   also stopped doing the tree serialization at a point because it was just taking too much time
-   ran out of 1TB of mem for (height 64, max<sub>thread</sub><sub>count</sub> 128, num<sub>entities</sub> 30000000)
-   adjusted for storing less nodes (set store depth to height/2 which is the default)
-   ran out of mem for (height 64, max<sub>thread</sub><sub>count</sub> 128, num<sub>entities</sub> 70000000)
-   adjusted for storing less nodes (set store depth to height/4)
-   ran up to 125M entities for height 64

Got another r7a.4xlarge

-   doing proof generation/verification
-   full store depth
-   all range proofs are aggregated (fastest)
-   no issues
