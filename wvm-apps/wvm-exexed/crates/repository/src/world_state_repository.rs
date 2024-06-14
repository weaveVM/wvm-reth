/*
    everything related to the repository stuff as an interface for data to be saved and retrieved from somewhere

    essentially:
        struct WorldStateRepository {
            bq_client,
        }

        WorldStateRepository interface {
            GetKnownTip(ctx) <tip(the most recent block), err>
            AddBlocksState(ctx, blocks vec<>) err
        }
 */