// store.js
import {createStore} from 'vuex';

export default createStore({
    state: {
        btcExchangeRate: 30000.0, // Default value
        ethExchangeRate: 2000.0 // Default value
    },
    getters: {
        // Getter for btcExchangeRate
        getBtcExchangeRate: (state) => {
            return state.btcExchangeRate;
        },
        getEthExchangeRate: (state) => {
            return state.ethExchangeRate;
        }
    },
    mutations: {
        // Mutation (setter) for btcExchangeRate
        setBtcExchangeRate(state, rate) {
            state.btcExchangeRate = rate;
        },
        setEthExchangeRate(state, rate) {
            state.ethExchangeRate = rate;
        }
    },
    actions: {
        // // Optional: Async action that could fetch and then commit the new rate
        // async fetchAndSetBtcExchangeRate({ commit }) {
        //     // Example using a fictional API endpoint
        //     // const response = await axios.get('https://api.example.com/btcRate');
        //     // commit('setBtcExchangeRate', response.data.rate);
        // }
    }
});
