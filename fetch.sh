owner=Homebrew
repo=homebrew-cask

mkdir data{,_monthly,_monthly_merged}/$owner
cargo run --release --bin gen_csv -- $owner $repo > data/$owner/$repo.csv &
cargo run --release --bin gen_csv_monthly -- $owner $repo > data_monthly/$owner/$repo.csv &
cargo run --release --bin gen_csv_monthly_merged -- $owner $repo > data_monthly_merged/$owner/$repo.csv &
wait