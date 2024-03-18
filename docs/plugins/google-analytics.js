export default defineNuxtPlugin((nuxtApp) => {
    // Check if we're in client mode, because Google Analytics only runs in the browser
    if (process.client) {
        // Load the Google Analytics script dynamically
        let script = document.createElement('script');
        script.async = true;
        script.src = 'https://www.googletagmanager.com/gtag/js?id=G-8MFHGQ6KVB';
        document.head.appendChild(script);

        // Initialize Google Analytics
        window.dataLayer = window.dataLayer || [];
        function gtag(){dataLayer.push(arguments);}
        gtag('js', new Date());

        // Your Google Analytics ID
        gtag('config', 'G-8MFHGQ6KVB');

        // Provide the gtag function globally in case you need to call Google Analytics events manually
        nuxtApp.provide('gtag', gtag);
    }
});

/*
<script async src="https://www.googletagmanager.com/gtag/js?id=G-8MFHGQ6KVB"></script>
      <script>
          window.dataLayer = window.dataLayer || [];
          function gtag(){dataLayer.push(arguments);}
          gtag('js', new Date());

          gtag('config', 'G-8MFHGQ6KVB');
      </script>
 */