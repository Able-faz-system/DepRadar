# Testnet Read Client

Minimal read-only client for the `deprader_router` Soroban contract on
Stellar testnet. Fetches a bounty via `get_bounty` and every claim
submitted against it via `get_claim`, and prints the result. No transactions are
submitted, no funded account is required — reads are simulated calls
against the RPC endpoint.

Default contract ID: `CCPRQANY467KKE4ROKCNW6TH3LY3EPVFRXIXJ5V4VCYHLJKWZPOZYQJQ`

## Usage

```bash
npm install
npm start                      # defaults to bounty-id 1
npm start -- --bounty-id 2
npm start -- --all             # every bounty, 1..bounty_count()
npm start -- --json            # single JSON document instead of text
npm start -- --all --json      # every bounty as one JSON document
```

Bounty `1` is real on-chain data: the `create_bounty` -> `submit_claim`
(x2) -> `reject_claim` -> `release` cycle recorded in
[PROGRESS.md](../PROGRESS.md), so it shows one rejected and one accepted
claim. Bounty `2` was cancelled and refunded. An unknown ID prints
`(no bounty with this ID)` rather than erroring, since `get_bounty` returns
`None` for unknown IDs on-chain.

### `--all`

Prints every bounty on-chain. The client first calls `bounty_count` and then
fetches bounties `1..count` (with their claims) in order. When combined with
`--bounty-id`, `--all` wins and the single-ID flag is ignored.

```bash
npm start -- --all
```

```text
Bounty #1
  creator:   G...
  ...
Bounty #2
  creator:   G...
  ...
```

### `--json`

Prints one JSON document instead of the human-readable output. With a single
bounty (the default, or `--bounty-id N`) the document is an object; with
`--all` it is an array of those objects, in bounty-ID order. Unknown IDs yield
`null` for the single-bounty case and are skipped by `--all`.

```bash
npm start -- --bounty-id 1 --json
```

```json
{
  "id": 1,
  "creator": "G...",
  "amount": "10000000",
  "status": "Released",
  "claims": [
    { "claimant": "G...", "status": "Rejected" },
    { "claimant": "G...", "status": "Accepted" }
  ]
}
```

```bash
npm start -- --all --json
```

```json
[
  { "id": 1, "creator": "G...", "amount": "10000000", "status": "Released", "claims": [] },
  { "id": 2, "creator": "G...", "amount": "5000000", "status": "Cancelled", "claims": [] }
]
```

## Configuration

The contract ID and RPC endpoint can be overridden without editing code, so
the client can be pointed at another deployment.

| Setting     | Flag         | Environment variable     | Default                                                        |
| ----------- | ------------ | ------------------------ | -------------------------------------------------------------- |
| Contract ID | `--contract` | `DEPRADAR_CONTRACT_ID`   | `CCPRQANY467KKE4ROKCNW6TH3LY3EPVFRXIXJ5V4VCYHLJKWZPOZYQJQ`     |
| RPC URL     | `--rpc`      | `DEPRADAR_RPC_URL`       | `https://soroban-testnet.stellar.org`                          |

Output options (no environment variable equivalents):

| Flag         | Effect                                                              |
| ------------ | ------------------------------------------------------------------- |
| `--bounty-id`| Fetch a single bounty (default `1`)                                 |
| `--all`      | Fetch every bounty using `bounty_count`; overrides `--bounty-id`    |
| `--json`     | Emit one JSON document instead of the human-readable text output    |

Precedence is **flag > environment variable > default**: a flag always wins,
otherwise the environment variable is used if set, otherwise the built-in
default applies.

### Examples

```bash
# Override both via flags
npm start -- --contract <CONTRACT_ID> --rpc <RPC_URL>

# Override via environment variables
DEPRADAR_CONTRACT_ID=<CONTRACT_ID> DEPRADAR_RPC_URL=<RPC_URL> npm start

# Flags take precedence over environment variables
DEPRADAR_CONTRACT_ID=<IGNORED_ID> npm start -- --contract <CONTRACT_ID>

# Combine with other options
npm start -- --bounty-id 2 --contract <CONTRACT_ID> --rpc <RPC_URL>

# Every bounty, as JSON, against a custom deployment
npm start -- --all --json --contract <CONTRACT_ID> --rpc <RPC_URL>
```

## Testing

The client uses [Vitest](https://vitest.dev/). Run the suite and the type
checker from `client/`:

```bash
npm test          # run the unit tests once
npm run typecheck # type-check without emitting
```

Argument parsing and output formatting live in `client/src/` as standalone,
exported functions so they can be tested without hitting the network:

- `parseArgs(argv, env)` resolves the effective configuration from CLI flags,
  environment variables and defaults (precedence **flag > env > default**). It
  throws on a missing or non-numeric `--bounty-id` and on an unknown flag.
- The formatter turns a bounty and its claims into the printed report,
  including the `(no bounty with this ID)` line for unknown IDs.
