using System.Collections;
using System.Collections.Generic;

public interface ActivateICardEffect
{
    Permanent PermanentWhenTriggered { get; set; }
    CardSource TopCardWhenTriggered { get; set; }
    IEnumerator Activate(Hashtable hash);
}
