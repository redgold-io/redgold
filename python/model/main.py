import networkx as nx

from ge import DeepWalk


def test_DeepWalk():
    G = nx.read_edgelist('./graph_data.csv',
                         create_using=nx.DiGraph(), nodetype=int,
                         data=False,
                         edgetype=float,
                         delimiter=",",

                         # , data=[('weight', int)]
                         )
    model = DeepWalk(G, walk_length=3, num_walks=2, workers=1)
    model.train(window_size=3, iter=1)
    embeddings = model.get_embeddings()
    for k, v in embeddings.items():
        print(k)
        print(v)
        print("--")
    # print(embeddings)


if __name__ == '__main__':
    test_DeepWalk()
