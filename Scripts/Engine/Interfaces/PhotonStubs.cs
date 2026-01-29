using UnityEngine;

public class MonoBehaviourPun : MonoBehaviour
{
    public PhotonView photonView;

    public virtual void OnEnable() {}
    public virtual void OnDisable() {}
}

public class MonoBehaviourPunCallbacks : MonoBehaviourPun
{
}

public class PhotonView : MonoBehaviour
{
    public int ViewID;
    public bool IsMine = true;
    public void RPC(string methodName, RpcTarget target, params object[] parameters) { }
}

public enum RpcTarget
{
    All,
    Others,
    MasterClient,
    AllBuffered,
    OthersBuffered
}

public class PunRPC : System.Attribute { }
