# Board State Tensor Layout

The `Board State Tensor` is a flattened `float[]` array (Size: 680) representing the current game state from the perspective of the requesting agent.

## Global Data [0-9]
| Index | Description | Notes |
| :--- | :--- | :--- |
| `0` | Turn Count | 1-based Global Turn Count |
| `1` | Current Phase | Enum Integer (0=Start, 1=Draw, 2=Breeding, 3=Main, 4=End, 5+=Pending) |
| `2` | Memory Gauge | Relative to player (Positive = My Turn Memory, Negative = Opponent Turn Memory) |
| `3` | Winner ID | 0=None, 1=P1, 2=P2 |
| `4-9` | Reserved | Padding |

## My Battle Area [10-249]
*Slots 0-11 (12 Slots * 20 Floats each)*
| Offset | Description | Notes |
| :--- | :--- | :--- |
| `+0` | Top Card ID | Internal Integer ID from `CardRegistry`. 0 if empty. |
| `+1` | Current DP | Digimon Power |
| `+2` | Is Suspended | `1.0` = Suspended, `0.0` = Active |
| `+3` | Source Count | Number of digivolution sources |
| `+4-19` | Source IDs | Bottom-to-Top list of up to 16 source card IDs. Padding 0. |

## Opponent Battle Area [250-489]
*Slots 0-11 (12 Slots * 20 Floats each)*
*   Same structure as My Battle Area.

## My Hand [490-509]
*   List of 20 Card IDs.
*   Padding 0.

## Opponent Hand [510-529]
*   List of 20 Card IDs.
*   *Note: In training, we assume perfect information access. In inference, this might be masked.*

## My Trash [530-574]
*   Top 45 Card IDs.

## Opponent Trash [575-619]
*   Top 45 Card IDs.

## My Security [620-629]
*   Top 10 Card IDs.

## Opponent Security [630-639]
*   Top 10 Card IDs.

## My Breeding Area [640-659]
*   1 Slot * 20 Floats (Same structure as Battle Slot).

## Opponent Breeding Area [660-679]
*   1 Slot * 20 Floats.
