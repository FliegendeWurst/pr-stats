owner=NixOS
repo=nixpkgs

mkdir data{,_monthly,_monthly_merged}/$owner
cargo build --release --bin gen_csv --bin gen_csv_monthly --bin gen_csv_monthly_merged
cargo run --release --bin gen_csv -- $owner $repo > data/$owner/$repo.csv &
cargo run --release --bin gen_csv_monthly -- $owner $repo > data_monthly/$owner/$repo.csv &
cargo run --release --bin gen_csv_monthly_merged -- $owner $repo > data_monthly_merged/$owner/$repo.csv &
wait
